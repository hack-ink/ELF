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

pub(super) async fn persist_extracted_note_add(
	tx: &mut Transaction<'_, Postgres>,
	args: PersistExtractedNoteArgs<'_>,
	note_id: Uuid,
	policy_decision: MemoryPolicyDecision,
) -> Result<AddEventPersistOutput> {
	access::ensure_active_project_scope_grant(
		&mut **tx,
		args.req.tenant_id.as_str(),
		args.project_id,
		args.scope,
		args.req.agent_id.as_str(),
	)
	.await?;

	let memory_note = MemoryNote {
		note_id,
		tenant_id: args.req.tenant_id.clone(),
		project_id: args.project_id.to_string(),
		agent_id: args.req.agent_id.clone(),
		scope: args.scope.to_string(),
		r#type: args.note_type.to_string(),
		key: args.key.map(ToString::to_string),
		text: args.text.to_string(),
		importance: args.importance,
		confidence: args.confidence,
		status: "active".to_string(),
		created_at: args.now,
		updated_at: args.now,
		expires_at: args.expires_at,
		embedding_version: args.embed_version.to_string(),
		source_ref: args.source_ref,
		hit_count: 0,
		last_hit_at: None,
	};

	persistence::insert_memory_note_tx(tx, &memory_note).await?;

	let note_version_id = crate::insert_version(
		&mut **tx,
		InsertVersionArgs {
			note_id: memory_note.note_id,
			op: "ADD",
			prev_snapshot: None,
			new_snapshot: Some(crate::note_snapshot(&memory_note)),
			reason: "add_event",
			actor: args.req.agent_id.as_str(),
			ts: args.now,
		},
	)
	.await?;

	crate::enqueue_outbox_tx(
		&mut **tx,
		memory_note.note_id,
		"UPSERT",
		args.embed_version,
		args.now,
	)
	.await?;
	persistence::upsert_structured_fields_tx(tx, args.structured, memory_note.note_id, args.now)
		.await?;

	if let Some(structured) = args.structured
		&& structured.has_graph_fields()
	{
		graph_ingestion::persist_graph_fields_tx(
			tx,
			args.req.tenant_id.as_str(),
			args.project_id,
			args.req.agent_id.as_str(),
			args.scope,
			memory_note.note_id,
			structured,
			args.now,
		)
		.await?;
	}

	Ok((
		AddEventResult {
			note_id: Some(note_id),
			op: NoteOp::Add,
			policy_decision,
			reason_code: None,
			reason: args.reason.cloned(),
			field_path: None,
			write_policy_audits: None,
		},
		Some(note_version_id),
	))
}
