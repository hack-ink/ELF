use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
	ElfService, InsertVersionArgs, Result, access,
	add_note::{
		materialize::structured_materialization,
		persistence::{self},
		types::{AddNoteContext, AddNoteInput},
	},
};
use elf_domain::ttl;
use elf_storage::models::MemoryNote;

pub(in crate::add_note) async fn handle_add_note_add(
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

	structured_materialization::upsert_structured_and_enqueue_outbox(
		tx,
		note,
		memory_note.note_id,
		ctx.embed_version,
		ctx.now,
	)
	.await?;
	structured_materialization::persist_graph_fields_if_present(
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
