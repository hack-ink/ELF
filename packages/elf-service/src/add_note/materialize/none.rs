use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	InsertVersionArgs, NoteOp, Result, access,
	add_note::{
		materialize::structured_materialization,
		types::{AddNoteContext, AddNoteInput, AddNoteResult},
	},
	structured_fields,
};
use elf_domain::memory_policy::MemoryPolicyDecision;
use elf_storage::models::MemoryNote;

#[allow(clippy::too_many_arguments)]
pub(in crate::add_note) async fn handle_add_note_none(
	tx: &mut Transaction<'_, Postgres>,
	ctx: &AddNoteContext<'_>,
	note: &AddNoteInput,
	note_id: Uuid,
	now: OffsetDateTime,
	embed_version: &str,
	policy_decision: MemoryPolicyDecision,
) -> Result<(AddNoteResult, Option<Uuid>)> {
	let mut should_update = false;

	if let Some(structured) = note.structured.as_ref() {
		if !structured.is_effectively_empty() {
			structured_fields::upsert_structured_fields_tx(tx, note_id, structured, now).await?;
			crate::enqueue_outbox_tx(&mut **tx, note_id, "UPSERT", embed_version, now).await?;

			should_update = true;
		}
		if structured.has_graph_fields() {
			structured_materialization::persist_graph_fields_if_present(
				tx,
				ctx.tenant_id,
				ctx.project_id,
				ctx.agent_id,
				ctx.scope,
				note_id,
				now,
				Some(structured),
			)
			.await?;

			should_update = true;
		}
	}

	if should_update {
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
				reason: "add_note_structured",
				actor: ctx.agent_id,
				ts: now,
			},
		)
		.await?;

		if matches!(ctx.scope, "project_shared" | "org_shared") {
			access::ensure_active_project_scope_grant(
				&mut **tx,
				ctx.tenant_id,
				ctx.project_id,
				ctx.scope,
				ctx.agent_id,
			)
			.await?;
		}

		return Ok((
			AddNoteResult {
				note_id: Some(note_id),
				op: NoteOp::Update,
				policy_decision,
				reason_code: None,
				field_path: None,
				write_policy_audit: None,
			},
			Some(note_version_id),
		));
	}

	Ok((
		AddNoteResult {
			note_id: Some(note_id),
			op: NoteOp::None,
			policy_decision,
			reason_code: None,
			field_path: None,
			write_policy_audit: None,
		},
		None,
	))
}
