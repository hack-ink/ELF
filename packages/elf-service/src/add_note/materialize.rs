use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	ElfService, InsertVersionArgs, NoteOp, Result, access,
	add_note::{
		persistence::{self},
		types::{AddNoteContext, AddNoteInput, AddNoteResult},
	},
	graph_ingestion,
	structured_fields::{self, StructuredFields},
};
use elf_domain::{memory_policy::MemoryPolicyDecision, ttl};
use elf_storage::models::MemoryNote;

pub(super) async fn handle_add_note_add(
	service: &ElfService,
	tx: &mut Transaction<'_, Postgres>,
	ctx: &AddNoteContext<'_>,
	note: &AddNoteInput,
	note_id: Uuid,
) -> Result<Uuid> {
	access::ensure_active_project_scope_grant(
		&mut **tx,
		ctx.tenant_id,
		ctx.project_id,
		ctx.scope,
		ctx.agent_id,
	)
	.await?;

	let expires_at =
		ttl::compute_expires_at(note.ttl_days, note.r#type.as_str(), &service.cfg, ctx.now);
	let memory_note = MemoryNote {
		note_id,
		tenant_id: ctx.tenant_id.to_string(),
		project_id: ctx.project_id.to_string(),
		agent_id: ctx.agent_id.to_string(),
		scope: ctx.scope.to_string(),
		r#type: note.r#type.clone(),
		key: note.key.clone(),
		text: note.text.clone(),
		importance: note.importance,
		confidence: note.confidence,
		status: "active".to_string(),
		created_at: ctx.now,
		updated_at: ctx.now,
		expires_at,
		embedding_version: ctx.embed_version.to_string(),
		source_ref: note.source_ref.clone(),
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
			reason: "add_note",
			actor: ctx.agent_id,
			ts: ctx.now,
		},
	)
	.await?;

	upsert_structured_and_enqueue_outbox(tx, note, memory_note.note_id, ctx.embed_version, ctx.now)
		.await?;
	persist_graph_fields_if_present(
		tx,
		ctx.tenant_id,
		ctx.project_id,
		ctx.agent_id,
		ctx.scope,
		memory_note.note_id,
		ctx.now,
		note.structured.as_ref(),
	)
	.await?;

	Ok(note_version_id)
}

pub(super) async fn handle_add_note_update(
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
	let requested_ttl = note.ttl_days.filter(|days| *days > 0);
	let expires_at = match requested_ttl {
		Some(ttl) => ttl::compute_expires_at(Some(ttl), note.r#type.as_str(), &service.cfg, now),
		None => existing.expires_at,
	};
	let expires_match = requested_ttl.map_or(existing.expires_at == expires_at, |ttl_days| {
		match existing.expires_at {
			Some(existing_expires_at) => {
				let existing_ttl = (existing_expires_at - existing.updated_at).whole_days() as i64;

				existing_ttl == ttl_days
			},
			None => false,
		}
	});
	let float_eps = 1e-6_f32;
	let unchanged = existing.text == note.text
		&& (existing.importance - note.importance).abs() <= float_eps
		&& (existing.confidence - note.confidence).abs() <= float_eps
		&& expires_match
		&& existing.source_ref == note.source_ref;

	if unchanged {
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

	persist_graph_fields_if_present(
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
	upsert_structured_and_enqueue_outbox(
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

#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_add_note_none(
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
			persist_graph_fields_if_present(
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

#[allow(clippy::too_many_arguments)]
async fn persist_graph_fields_if_present(
	tx: &mut Transaction<'_, Postgres>,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	scope: &str,
	note_id: Uuid,
	now: OffsetDateTime,
	structured: Option<&StructuredFields>,
) -> Result<()> {
	let Some(structured) = structured else {
		return Ok(());
	};

	if !structured.has_graph_fields() {
		return Ok(());
	}

	graph_ingestion::persist_graph_fields_tx(
		tx, tenant_id, project_id, agent_id, scope, note_id, structured, now,
	)
	.await?;

	Ok(())
}

async fn upsert_structured_and_enqueue_outbox(
	tx: &mut Transaction<'_, Postgres>,
	note: &AddNoteInput,
	note_id: Uuid,
	embed_version: &str,
	now: OffsetDateTime,
) -> Result<()> {
	if let Some(structured) = note.structured.as_ref()
		&& !structured.is_effectively_empty()
	{
		structured_fields::upsert_structured_fields_tx(tx, note_id, structured, now).await?;
	}

	crate::enqueue_outbox_tx(&mut **tx, note_id, "UPSERT", embed_version, now).await?;

	Ok(())
}
