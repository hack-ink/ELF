use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
	InsertVersionArgs, NoteOp, Result, access,
	add_event::types::{AddEventPersistOutput, AddEventResult, PersistExtractedNoteArgs},
	graph_ingestion,
	structured_fields::{self, StructuredFields},
};
use elf_domain::memory_policy::MemoryPolicyDecision;
use elf_storage::models::MemoryNote;

pub(super) async fn persist_extracted_note_none(
	tx: &mut Transaction<'_, Postgres>,
	args: PersistExtractedNoteArgs<'_>,
	note_id: Uuid,
	policy_decision: MemoryPolicyDecision,
) -> Result<AddEventPersistOutput> {
	let Some(structured) = args.structured else {
		return Ok(none_result(note_id, policy_decision, args.reason.cloned()));
	};

	if !structured_requires_update(structured) {
		return Ok(none_result(note_id, policy_decision, args.reason.cloned()));
	}
	if !structured.is_effectively_empty() {
		structured_fields::upsert_structured_fields_tx(tx, note_id, structured, args.now).await?;
		crate::enqueue_outbox_tx(&mut **tx, note_id, "UPSERT", args.embed_version, args.now)
			.await?;
	}
	if structured.has_graph_fields() {
		graph_ingestion::persist_graph_fields_tx(
			tx,
			args.req.tenant_id.as_str(),
			args.project_id,
			args.req.agent_id.as_str(),
			args.scope,
			note_id,
			structured,
			args.now,
		)
		.await?;
	}

	let note_row: MemoryNote = sqlx::query_as("SELECT * FROM memory_notes WHERE note_id = $1")
		.bind(note_id)
		.fetch_one(&mut **tx)
		.await?;
	let snapshot = crate::note_snapshot(&note_row);
	let note_version_id = crate::insert_version(
		&mut **tx,
		InsertVersionArgs {
			note_id,
			op: "UPDATE",
			prev_snapshot: Some(snapshot.clone()),
			new_snapshot: Some(snapshot),
			reason: "add_event_structured",
			actor: args.req.agent_id.as_str(),
			ts: args.now,
		},
	)
	.await?;

	if matches!(args.scope, "project_shared" | "org_shared") {
		access::ensure_active_project_scope_grant(
			&mut **tx,
			args.req.tenant_id.as_str(),
			args.project_id,
			args.scope,
			args.req.agent_id.as_str(),
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

fn none_result(
	note_id: Uuid,
	policy_decision: MemoryPolicyDecision,
	reason: Option<String>,
) -> AddEventPersistOutput {
	(
		AddEventResult {
			note_id: Some(note_id),
			op: NoteOp::None,
			policy_decision,
			reason_code: None,
			reason,
			field_path: None,
			write_policy_audits: None,
		},
		None,
	)
}

fn structured_requires_update(structured: &StructuredFields) -> bool {
	!structured.is_effectively_empty() || structured.has_graph_fields()
}

#[cfg(test)] mod tests;
