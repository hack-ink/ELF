use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	ElfService, InsertVersionArgs, NoteOp, Result, access,
	add_note::{
		materialize::structured_materialization,
		persistence::{self},
		types::{AddNoteInput, AddNoteResult},
	},
};
use elf_domain::{memory_policy::MemoryPolicyDecision, ttl};
use elf_storage::models::MemoryNote;

pub(in crate::add_note) async fn handle_add_note_update(
	service: &ElfService,
	tx: &mut Transaction<'_, Postgres>,
	note: &AddNoteInput,
	note_id: Uuid,
	agent_id: &str,
	now: OffsetDateTime,
	policy_decision: MemoryPolicyDecision,
) -> Result<(AddNoteResult, Option<Uuid>)> {
	let mut existing: MemoryNote =
		sqlx::query_as::<_, MemoryNote>("SELECT * FROM memory_notes WHERE note_id = $1 FOR UPDATE")
			.bind(note_id)
			.fetch_one(&mut **tx)
			.await?;
	let prev_snapshot = crate::note_snapshot(&existing);
	let requested_ttl = requested_ttl_days(note);
	let expires_at = match requested_ttl {
		Some(ttl) => ttl::compute_expires_at(Some(ttl), note.r#type.as_str(), &service.cfg, now),
		None => existing.expires_at,
	};

	if note_update_is_unchanged(&existing, note, expires_at, requested_ttl) {
		return Ok((
			AddNoteResult {
				note_id: Some(note_id),
				op: NoteOp::None,
				policy_decision: MemoryPolicyDecision::Ignore,
				reason_code: None,
				field_path: None,
				write_policy_audit: None,
			},
			None,
		));
	}

	access::ensure_active_project_scope_grant(
		&mut **tx,
		existing.tenant_id.as_str(),
		existing.project_id.as_str(),
		existing.scope.as_str(),
		existing.agent_id.as_str(),
	)
	.await?;

	existing.text = note.text.clone();
	existing.importance = note.importance;
	existing.confidence = note.confidence;
	existing.updated_at = now;
	existing.expires_at = expires_at;
	existing.source_ref = note.source_ref.clone();

	persistence::update_memory_note_tx(tx, &existing).await?;

	let note_version_id = crate::insert_version(
		&mut **tx,
		InsertVersionArgs {
			note_id: existing.note_id,
			op: "UPDATE",
			prev_snapshot: Some(prev_snapshot),
			new_snapshot: Some(crate::note_snapshot(&existing)),
			reason: "add_note",
			actor: agent_id,
			ts: now,
		},
	)
	.await?;

	structured_materialization::persist_graph_fields_if_present(
		tx,
		existing.tenant_id.as_str(),
		existing.project_id.as_str(),
		existing.agent_id.as_str(),
		existing.scope.as_str(),
		existing.note_id,
		now,
		note.structured.as_ref(),
	)
	.await?;
	structured_materialization::upsert_structured_and_enqueue_outbox(
		tx,
		note,
		existing.note_id,
		existing.embedding_version.as_str(),
		now,
	)
	.await?;

	Ok((
		AddNoteResult {
			note_id: Some(note_id),
			op: NoteOp::Update,
			policy_decision,
			reason_code: None,
			field_path: None,
			write_policy_audit: None,
		},
		Some(note_version_id),
	))
}

fn requested_ttl_days(note: &AddNoteInput) -> Option<i64> {
	note.ttl_days.filter(|days| *days > 0)
}

fn note_update_is_unchanged(
	existing: &MemoryNote,
	note: &AddNoteInput,
	expires_at: Option<OffsetDateTime>,
	requested_ttl: Option<i64>,
) -> bool {
	let expires_match = requested_ttl.map_or(existing.expires_at == expires_at, |ttl_days| {
		match existing.expires_at {
			Some(existing_expires_at) => {
				let existing_ttl = (existing_expires_at - existing.updated_at).whole_days();

				existing_ttl == ttl_days
			},
			None => false,
		}
	});
	let float_eps = 1e-6_f32;

	existing.text == note.text
		&& (existing.importance - note.importance).abs() <= float_eps
		&& (existing.confidence - note.confidence).abs() <= float_eps
		&& expires_match
		&& existing.source_ref == note.source_ref
}

#[cfg(test)] mod tests;
