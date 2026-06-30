use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
	InsertVersionArgs, NoteOp, Result, access,
	add_event::{
		persistence::{self},
		types::{AddEventPersistOutput, AddEventResult, PersistExtractedNoteArgs},
	},
	graph_ingestion,
};
use elf_domain::memory_policy::MemoryPolicyDecision;
use elf_storage::models::MemoryNote;

pub(super) async fn persist_extracted_note_update(
	tx: &mut Transaction<'_, Postgres>,
	args: PersistExtractedNoteArgs<'_>,
	note_id: Uuid,
	policy_decision: MemoryPolicyDecision,
) -> Result<AddEventPersistOutput> {
	let mut existing: MemoryNote =
		sqlx::query_as::<_, MemoryNote>("SELECT * FROM memory_notes WHERE note_id = $1 FOR UPDATE")
			.bind(note_id)
			.fetch_one(&mut **tx)
			.await?;

	access::ensure_active_project_scope_grant(
		&mut **tx,
		existing.tenant_id.as_str(),
		existing.project_id.as_str(),
		existing.scope.as_str(),
		existing.agent_id.as_str(),
	)
	.await?;

	let prev_snapshot = crate::note_snapshot(&existing);

	existing.text = args.text.to_string();
	existing.importance = args.importance;
	existing.confidence = args.confidence;
	existing.updated_at = args.now;
	existing.expires_at = args.expires_at;
	existing.source_ref = args.source_ref;

	persistence::update_memory_note_tx(tx, &existing).await?;

	let note_version_id = crate::insert_version(
		&mut **tx,
		InsertVersionArgs {
			note_id: existing.note_id,
			op: "UPDATE",
			prev_snapshot: Some(prev_snapshot),
			new_snapshot: Some(crate::note_snapshot(&existing)),
			reason: "add_event",
			actor: args.req.agent_id.as_str(),
			ts: args.now,
		},
	)
	.await?;

	crate::enqueue_outbox_tx(
		&mut **tx,
		existing.note_id,
		"UPSERT",
		existing.embedding_version.as_str(),
		args.now,
	)
	.await?;
	persistence::upsert_structured_fields_tx(tx, args.structured, existing.note_id, args.now)
		.await?;

	if let Some(structured) = args.structured
		&& structured.has_graph_fields()
	{
		graph_ingestion::persist_graph_fields_tx(
			tx,
			args.req.tenant_id.as_str(),
			existing.project_id.as_str(),
			args.req.agent_id.as_str(),
			args.scope,
			existing.note_id,
			structured,
			args.now,
		)
		.await?;
	}

	Ok((
		AddEventResult {
			note_id: Some(note_id),
			op: NoteOp::Update,
			policy_decision,
			reason_code: None,
			reason: args.reason.cloned(),
			field_path: None,
			write_policy_audits: None,
		},
		Some(note_version_id),
	))
}
