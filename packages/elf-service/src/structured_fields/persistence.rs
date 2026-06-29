use std::{collections::HashMap, slice};

use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Result;

use super::types::StructuredFields;

/// Upserts summary, fact, and concept fields for one note inside an existing transaction.
pub async fn upsert_structured_fields_tx(
	executor: &mut PgConnection,
	note_id: Uuid,
	structured: &StructuredFields,
	now: OffsetDateTime,
) -> Result<()> {
	if let Some(summary) = structured.summary.as_ref() {
		replace_kind(executor, note_id, "summary", slice_single(summary), now).await?;
	}
	if let Some(facts) = structured.facts.as_ref() {
		replace_kind(executor, note_id, "fact", facts.as_slice(), now).await?;
	}
	if let Some(concepts) = structured.concepts.as_ref() {
		replace_kind(executor, note_id, "concept", concepts.as_slice(), now).await?;
	}

	Ok(())
}

/// Fetches persisted structured fields for the provided note identifiers.
pub async fn fetch_structured_fields(
	pool: &PgPool,
	note_ids: &[Uuid],
) -> Result<HashMap<Uuid, StructuredFields>> {
	if note_ids.is_empty() {
		return Ok(HashMap::new());
	}

	let rows = sqlx::query_as::<_, (Uuid, String, i32, String)>(
		"\
SELECT
	note_id,
	field_kind,
	item_index,
	text
FROM memory_note_fields
WHERE note_id = ANY($1::uuid[])
ORDER BY note_id ASC, field_kind ASC, item_index ASC",
	)
	.bind(note_ids.to_vec())
	.fetch_all(pool)
	.await?;
	let mut out: HashMap<Uuid, StructuredFields> = HashMap::new();

	for row in rows {
		let (note_id, field_kind, _item_index, text) = row;
		let entry = out.entry(note_id).or_default();

		match field_kind.as_str() {
			"summary" =>
				if entry.summary.is_none() && !text.trim().is_empty() {
					entry.summary = Some(text);
				},
			"fact" => {
				entry.facts.get_or_insert_with(Vec::new).push(text);
			},
			"concept" => {
				entry.concepts.get_or_insert_with(Vec::new).push(text);
			},
			_ => {},
		}
	}

	out.retain(|_, value| !value.is_effectively_empty());

	Ok(out)
}

fn slice_single(value: &String) -> &[String] {
	slice::from_ref(value)
}

async fn replace_kind(
	executor: &mut PgConnection,
	note_id: Uuid,
	kind: &str,
	items: &[String],
	now: OffsetDateTime,
) -> Result<()> {
	sqlx::query("DELETE FROM memory_note_fields WHERE note_id = $1 AND field_kind = $2")
		.bind(note_id)
		.bind(kind)
		.execute(&mut *executor)
		.await?;

	for (idx, value) in items.iter().enumerate() {
		let trimmed = value.trim();

		if trimmed.is_empty() {
			continue;
		}

		sqlx::query(
			"\
INSERT INTO memory_note_fields (
	field_id,
	note_id,
	field_kind,
	item_index,
	text,
	created_at,
	updated_at
)
VALUES ($1,$2,$3,$4,$5,$6,$7)",
		)
		.bind(Uuid::new_v4())
		.bind(note_id)
		.bind(kind)
		.bind(idx as i32)
		.bind(trimmed)
		.bind(now)
		.bind(now)
		.execute(&mut *executor)
		.await?;
	}

	Ok(())
}
