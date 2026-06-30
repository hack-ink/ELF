use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	Result,
	add_note::types::AddNoteInput,
	graph_ingestion,
	structured_fields::{self, StructuredFields},
};

#[allow(clippy::too_many_arguments)]
pub(super) async fn persist_graph_fields_if_present(
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

pub(super) async fn upsert_structured_and_enqueue_outbox(
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
