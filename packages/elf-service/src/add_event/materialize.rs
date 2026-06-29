use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
	InsertVersionArgs, NoteOp, Result, UpdateDecision, access,
	add_event::{
		persistence::{self},
		types::{AddEventPersistOutput, AddEventResult, PersistExtractedNoteArgs},
	},
	graph_ingestion, structured_fields,
};
use elf_domain::memory_policy::MemoryPolicyDecision;
use elf_storage::models::MemoryNote;

pub(super) async fn persist_extracted_note_decision(
	tx: &mut Transaction<'_, Postgres>,
	args: PersistExtractedNoteArgs<'_>,
	decision: UpdateDecision,
	policy_decision: MemoryPolicyDecision,
) -> Result<AddEventPersistOutput> {
	match (decision, args) {
		(UpdateDecision::Add { note_id, .. }, args) =>
			persist_extracted_note_add(tx, args, note_id, policy_decision).await,
		(UpdateDecision::Update { note_id, .. }, args) =>
			persist_extracted_note_update(tx, args, note_id, policy_decision).await,
		(UpdateDecision::None { note_id, .. }, args) =>
			persist_extracted_note_none(tx, args, note_id, policy_decision).await,
	}
}

async fn persist_extracted_note_add(
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

async fn persist_extracted_note_update(
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

async fn persist_extracted_note_none(
	tx: &mut Transaction<'_, Postgres>,
	args: PersistExtractedNoteArgs<'_>,
	note_id: Uuid,
	policy_decision: MemoryPolicyDecision,
) -> Result<AddEventPersistOutput> {
	let mut did_update = false;

	if let Some(structured) = args.structured
		&& !structured.is_effectively_empty()
	{
		structured_fields::upsert_structured_fields_tx(tx, note_id, structured, args.now).await?;
		crate::enqueue_outbox_tx(&mut **tx, note_id, "UPSERT", args.embed_version, args.now)
			.await?;

		did_update = true;
	}
	if let Some(structured) = args.structured
		&& structured.has_graph_fields()
	{
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

		did_update = true;
	}

	if did_update {
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

		return Ok((
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
		));
	}

	Ok((
		AddEventResult {
			note_id: Some(note_id),
			op: NoteOp::None,
			policy_decision,
			reason_code: None,
			reason: args.reason.cloned(),
			field_path: None,
			write_policy_audits: None,
		},
		None,
	))
}
