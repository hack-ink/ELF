use std::collections::HashMap;

use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use elf_domain::{cjk, evidence};

use crate::{Error, Result};

const MAX_LIST_ITEMS: usize = 64;
const MAX_ITEM_CHARS: usize = 1_000;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct StructuredFields {
	pub summary: Option<String>,
	pub facts: Option<Vec<String>>,
	pub concepts: Option<Vec<String>>,
}

impl StructuredFields {
	pub fn is_effectively_empty(&self) -> bool {
		let summary_empty = self.summary.as_ref().map(|v| v.trim().is_empty()).unwrap_or(true);
		let facts_empty = self
			.facts
			.as_ref()
			.map(|items| items.iter().all(|v| v.trim().is_empty()))
			.unwrap_or(true);
		let concepts_empty = self
			.concepts
			.as_ref()
			.map(|items| items.iter().all(|v| v.trim().is_empty()))
			.unwrap_or(true);

		summary_empty && facts_empty && concepts_empty
	}
}

#[derive(Debug, Clone, serde::Deserialize)]
struct SourceRefEvidenceQuote {
	quote: String,
}

pub fn validate_structured_fields(
	structured: &StructuredFields,
	note_text: &str,
	source_ref: &Value,
	add_event_evidence: Option<&[(usize, String)]>,
) -> Result<()> {
	if let Some(summary) = structured.summary.as_ref() {
		validate_text_field(summary, "structured.summary")?;
	}
	if let Some(facts) = structured.facts.as_ref() {
		validate_list_field(facts, "structured.facts")?;

		let evidence_quotes: Vec<String> = if let Some(event_evidence) = add_event_evidence {
			event_evidence.iter().map(|(_, quote)| quote.clone()).collect()
		} else {
			extract_source_ref_quotes(source_ref)
		};

		for (idx, fact) in facts.iter().enumerate() {
			validate_text_field(fact, &format!("structured.facts[{idx}]"))?;
			if !fact_is_evidence_bound(fact, note_text, &evidence_quotes) {
				return Err(Error::InvalidRequest {
					message: format!(
						"structured.facts[{idx}] is not supported by note text or evidence quotes."
					),
				});
			}
		}
	}
	if let Some(concepts) = structured.concepts.as_ref() {
		validate_list_field(concepts, "structured.concepts")?;
		for (idx, concept) in concepts.iter().enumerate() {
			validate_text_field(concept, &format!("structured.concepts[{idx}]"))?;
		}
	}

	Ok(())
}

fn validate_list_field(items: &[String], label: &str) -> Result<()> {
	if items.len() > MAX_LIST_ITEMS {
		return Err(Error::InvalidRequest {
			message: format!("{label} must have at most {MAX_LIST_ITEMS} items."),
		});
	}
	Ok(())
}

fn validate_text_field(value: &str, label: &str) -> Result<()> {
	let trimmed = value.trim();
	if trimmed.is_empty() {
		return Err(Error::InvalidRequest { message: format!("{label} must not be empty.") });
	}
	if trimmed.chars().count() > MAX_ITEM_CHARS {
		return Err(Error::InvalidRequest {
			message: format!("{label} must be at most {MAX_ITEM_CHARS} characters."),
		});
	}
	if cjk::contains_cjk(trimmed) {
		return Err(Error::NonEnglishInput { field: label.to_string() });
	}
	Ok(())
}

fn extract_source_ref_quotes(source_ref: &Value) -> Vec<String> {
	let Some(evidence) = source_ref.get("evidence") else {
		return Vec::new();
	};
	let Ok(quotes) = serde_json::from_value::<Vec<SourceRefEvidenceQuote>>(evidence.clone()) else {
		return Vec::new();
	};
	quotes.into_iter().map(|q| q.quote).collect()
}

fn fact_is_evidence_bound(fact: &str, note_text: &str, evidence_quotes: &[String]) -> bool {
	let trimmed = fact.trim();
	if trimmed.is_empty() {
		return false;
	}
	if note_text.contains(trimmed) {
		return true;
	}
	for quote in evidence_quotes {
		if quote.contains(trimmed) {
			return true;
		}
	}
	false
}

pub fn event_evidence_quotes(messages: &[String], evidence: &[(usize, String)]) -> Result<()> {
	for (idx, (message_index, quote)) in evidence.iter().enumerate() {
		if quote.trim().is_empty() {
			return Err(Error::InvalidRequest {
				message: format!("evidence[{idx}].quote must not be empty."),
			});
		}
		if !evidence::evidence_matches(messages, *message_index, quote) {
			return Err(Error::InvalidRequest {
				message: format!("evidence[{idx}] does not match its source message."),
			});
		}
	}
	Ok(())
}

pub async fn upsert_structured_fields_tx(
	executor: &mut sqlx::PgConnection,
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

fn slice_single(value: &String) -> &[String] {
	std::slice::from_ref(value)
}

async fn replace_kind(
	executor: &mut sqlx::PgConnection,
	note_id: Uuid,
	kind: &str,
	items: &[String],
	now: OffsetDateTime,
) -> Result<()> {
	sqlx::query!(
		"DELETE FROM memory_note_fields WHERE note_id = $1 AND field_kind = $2",
		note_id,
		kind,
	)
	.execute(&mut *executor)
	.await?;

	for (idx, value) in items.iter().enumerate() {
		let trimmed = value.trim();
		if trimmed.is_empty() {
			continue;
		}
		sqlx::query!(
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
			Uuid::new_v4(),
			note_id,
			kind,
			idx as i32,
			trimmed,
			now,
			now,
		)
		.execute(&mut *executor)
		.await?;
	}

	Ok(())
}

pub async fn fetch_structured_fields(
	pool: &sqlx::PgPool,
	note_ids: &[Uuid],
) -> Result<HashMap<Uuid, StructuredFields>> {
	if note_ids.is_empty() {
		return Ok(HashMap::new());
	}

	let rows = sqlx::query!(
		"\
SELECT
	note_id AS \"note_id!\",
	field_kind AS \"field_kind!\",
	item_index AS \"item_index!\",
	text AS \"text!\"
FROM memory_note_fields
WHERE note_id = ANY($1::uuid[])
ORDER BY note_id ASC, field_kind ASC, item_index ASC",
		note_ids,
	)
	.fetch_all(pool)
	.await?;

	let mut out: HashMap<Uuid, StructuredFields> = HashMap::new();

	for row in rows {
		let entry = out.entry(row.note_id).or_insert_with(StructuredFields::default);
		match row.field_kind.as_str() {
			"summary" =>
				if entry.summary.is_none() && !row.text.trim().is_empty() {
					entry.summary = Some(row.text);
				},
			"fact" => {
				entry.facts.get_or_insert_with(Vec::new).push(row.text);
			},
			"concept" => {
				entry.concepts.get_or_insert_with(Vec::new).push(row.text);
			},
			_ => {},
		}
	}

	out.retain(|_, value| !value.is_effectively_empty());

	Ok(out)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fact_binding_accepts_note_text_substring() {
		let structured = StructuredFields {
			summary: None,
			facts: Some(vec!["Deploy uses reranking".to_string()]),
			concepts: None,
		};
		let res = validate_structured_fields(
			&structured,
			"Deploy uses reranking after retrieval.",
			&serde_json::json!({}),
			None,
		);
		assert!(res.is_ok());
	}

	#[test]
	fn fact_binding_rejects_without_text_or_evidence() {
		let structured = StructuredFields {
			summary: None,
			facts: Some(vec!["Nonexistent claim.".to_string()]),
			concepts: None,
		};
		let res =
			validate_structured_fields(&structured, "Some note.", &serde_json::json!({}), None);
		assert!(res.is_err());
	}
}
