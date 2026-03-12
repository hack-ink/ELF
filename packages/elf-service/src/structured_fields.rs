//! Structured-field validation and persistence helpers.

use std::{collections::HashMap, slice};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, Result};
use elf_domain::{english_gate, evidence};

const MAX_LIST_ITEMS: usize = 64;
const MAX_ENTITIES: usize = 32;
const MAX_RELATIONS: usize = 64;
const MAX_ALIASES: usize = 16;
const MAX_ITEM_CHARS: usize = 1_000;

/// Structured note fields emitted by extraction and stored alongside a note.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct StructuredFields {
	/// Optional one-paragraph summary.
	pub summary: Option<String>,
	/// Optional fact statements grounded in the note text.
	pub facts: Option<Vec<String>>,
	/// Optional concept labels grounded in the note text.
	pub concepts: Option<Vec<String>>,
	/// Optional graph entities extracted from the note.
	pub entities: Option<Vec<StructuredEntity>>,
	/// Optional graph relations extracted from the note.
	pub relations: Option<Vec<StructuredRelation>>,
}
impl StructuredFields {
	/// Returns `true` when no persisted summary, fact, or concept content is present.
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

	/// Returns `true` when graph entities or relations are present.
	pub fn has_graph_fields(&self) -> bool {
		self.entities.as_ref().is_some_and(|entities| !entities.is_empty())
			|| self.relations.as_ref().is_some_and(|relations| !relations.is_empty())
	}
}

/// One extracted entity candidate.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct StructuredEntity {
	/// Canonical surface for the entity.
	pub canonical: Option<String>,
	/// Optional entity kind such as person or organization.
	pub kind: Option<String>,
	/// Optional alternate surfaces for the entity.
	pub aliases: Option<Vec<String>>,
}

/// One extracted relation candidate.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct StructuredRelation {
	/// Relation subject entity.
	pub subject: Option<StructuredEntity>,
	/// Predicate surface for the relation.
	pub predicate: Option<String>,
	/// Relation object, either an entity or scalar value.
	pub object: Option<StructuredRelationObject>,
	#[serde(with = "crate::time_serde::option")]
	/// Optional validity-window start.
	pub valid_from: Option<OffsetDateTime>,
	#[serde(with = "crate::time_serde::option")]
	/// Optional validity-window end.
	pub valid_to: Option<OffsetDateTime>,
}

/// Extracted relation object.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct StructuredRelationObject {
	/// Entity-shaped object value.
	pub entity: Option<StructuredEntity>,
	/// Scalar object value.
	pub value: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct SourceRefEvidenceQuote {
	quote: String,
}

/// Validates structured fields against note text, evidence bindings, and size limits.
pub fn validate_structured_fields(
	structured: &StructuredFields,
	note_text: &str,
	source_ref: &Value,
	add_event_evidence: Option<&[(usize, String)]>,
) -> Result<()> {
	let evidence_quotes: Vec<String> = if let Some(event_evidence) = add_event_evidence {
		event_evidence.iter().map(|(_, quote)| quote.clone()).collect()
	} else {
		extract_source_ref_quotes(source_ref)
	};

	if let Some(summary) = structured.summary.as_ref() {
		validate_text_field(summary, "structured.summary")?;
	}
	if let Some(entities) = structured.entities.as_ref() {
		validate_list_field_count(entities.len(), MAX_ENTITIES, "structured.entities")?;

		for (idx, entity) in entities.iter().enumerate() {
			let base = format!("structured.entities[{idx}]");

			validate_structured_entity(entity, &base, true)?;
		}
	}
	if let Some(relations) = structured.relations.as_ref() {
		validate_list_field_count(relations.len(), MAX_RELATIONS, "structured.relations")?;

		for (idx, relation) in relations.iter().enumerate() {
			validate_structured_relation(
				relation,
				note_text,
				&evidence_quotes,
				&format!("structured.relations[{idx}]"),
			)?;
		}
	}
	if let Some(facts) = structured.facts.as_ref() {
		validate_list_field(facts, "structured.facts")?;

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

/// Validates event-evidence quotes against their source messages.
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

fn validate_structured_entity(
	entity: &StructuredEntity,
	base: &str,
	require_canonical: bool,
) -> Result<()> {
	if require_canonical {
		validate_required_text_field(entity.canonical.as_ref(), &format!("{base}.canonical"))?;
	}

	if let Some(kind) = entity.kind.as_ref() {
		validate_text_field(kind, &format!("{base}.kind"))?;
	}
	if let Some(aliases) = entity.aliases.as_ref() {
		validate_list_field_count(aliases.len(), MAX_ALIASES, &format!("{base}.aliases"))?;

		for (alias_idx, alias) in aliases.iter().enumerate() {
			validate_text_field(alias, &format!("{base}.aliases[{alias_idx}]"))?;
		}
	}

	Ok(())
}

fn validate_structured_relation(
	relation: &StructuredRelation,
	note_text: &str,
	evidence_quotes: &[String],
	base: &str,
) -> Result<()> {
	if relation.predicate.is_none() {
		return Err(Error::InvalidRequest { message: format!("{base}.predicate is required.") });
	}

	let subject = relation
		.subject
		.as_ref()
		.ok_or_else(|| Error::InvalidRequest { message: format!("{base}.subject is required.") })?;

	validate_structured_entity(subject, &format!("{base}.subject"), true)?;

	let predicate = relation.predicate.as_ref().ok_or_else(|| Error::InvalidRequest {
		message: format!("{base}.predicate is required."),
	})?;

	validate_text_field(predicate, &format!("{base}.predicate"))?;

	let object = relation
		.object
		.as_ref()
		.ok_or_else(|| Error::InvalidRequest { message: format!("{base}.object is required.") })?;

	match (&object.entity, object.value.as_ref()) {
		(Some(entity), None) => {
			validate_structured_entity(entity, &format!("{base}.object.entity"), true)?;

			let canonical = entity.canonical.as_deref().ok_or_else(|| Error::InvalidRequest {
				message: format!("{base}.object.entity.canonical is required."),
			})?;

			if !fact_is_evidence_bound(canonical, note_text, evidence_quotes) {
				return Err(Error::InvalidRequest {
					message: format!(
						"{base}.object.entity.canonical is not supported by note text or evidence quotes."
					),
				});
			}
		},
		(None, Some(value)) => {
			validate_text_field(value, &format!("{base}.object.value"))?;

			if !fact_is_evidence_bound(value, note_text, evidence_quotes) {
				return Err(Error::InvalidRequest {
					message: format!(
						"{base}.object.value is not supported by note text or evidence quotes."
					),
				});
			}
		},
		(_, _) => {
			return Err(Error::InvalidRequest {
				message: format!("{base}.object must provide exactly one of entity or value."),
			});
		},
	}

	if !fact_is_evidence_bound(
		subject.canonical.as_deref().unwrap_or_default(),
		note_text,
		evidence_quotes,
	) {
		return Err(Error::InvalidRequest {
			message: format!(
				"{base}.subject.canonical is not supported by note text or evidence quotes."
			),
		});
	}
	if !fact_is_evidence_bound(predicate, note_text, evidence_quotes) {
		return Err(Error::InvalidRequest {
			message: format!("{base}.predicate is not supported by note text or evidence quotes."),
		});
	}

	if let (Some(valid_from), Some(valid_to)) = (relation.valid_from, relation.valid_to)
		&& valid_to <= valid_from
	{
		return Err(Error::InvalidRequest {
			message: format!("{base}.valid_to must be greater than valid_from."),
		});
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
	if !english_gate::is_english_natural_language(trimmed) {
		return Err(Error::NonEnglishInput { field: label.to_string() });
	}

	Ok(())
}

fn validate_required_text_field(value: Option<&String>, label: &str) -> Result<()> {
	let Some(value) = value else {
		return Err(Error::InvalidRequest { message: format!("{label} is required.") });
	};

	validate_text_field(value, label)
}

fn validate_list_field_count(len: usize, max: usize, label: &str) -> Result<()> {
	if len > max {
		return Err(Error::InvalidRequest {
			message: format!("{label} must have at most {max} items."),
		});
	}

	Ok(())
}

fn extract_source_ref_quotes(source_ref: &Value) -> Vec<String> {
	let Some(evidence) = source_ref.get("evidence") else { return Vec::new() };
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

#[cfg(test)]
mod tests {
	use time::OffsetDateTime;

	use crate::{
		Error,
		structured_fields::{
			self, StructuredEntity, StructuredFields, StructuredRelation, StructuredRelationObject,
		},
	};

	fn structured_relation(
		subject: &str,
		predicate: &str,
		object: StructuredRelationObject,
		valid_from: Option<OffsetDateTime>,
		valid_to: Option<OffsetDateTime>,
	) -> StructuredFields {
		StructuredFields {
			summary: None,
			facts: None,
			concepts: None,
			entities: None,
			relations: Some(vec![StructuredRelation {
				subject: Some(StructuredEntity {
					canonical: Some(subject.to_string()),
					kind: None,
					aliases: None,
				}),
				predicate: Some(predicate.to_string()),
				object: Some(object),
				valid_from,
				valid_to,
			}]),
		}
	}

	#[test]
	fn fact_binding_accepts_note_text_substring() {
		let structured = StructuredFields {
			summary: None,
			facts: Some(vec!["Deploy uses reranking".to_string()]),
			concepts: None,
			entities: None,
			relations: None,
		};
		let res = structured_fields::validate_structured_fields(
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
			entities: None,
			relations: None,
		};
		let res = structured_fields::validate_structured_fields(
			&structured,
			"Some note.",
			&serde_json::json!({}),
			None,
		);

		assert!(res.is_err());
	}

	#[test]
	fn relation_object_requires_exactly_one_of_entity_or_value() {
		let structured = structured_relation(
			"alice",
			"owns",
			StructuredRelationObject {
				entity: Some(StructuredEntity {
					canonical: Some("Acme".to_string()),
					kind: None,
					aliases: None,
				}),
				value: Some("Acme corp".to_string()),
			},
			None,
			None,
		);
		let res = structured_fields::validate_structured_fields(
			&structured,
			"alice owns Acme corp.",
			&serde_json::json!({
				"evidence": [{"quote": "alice owns Acme"}]
			}),
			None,
		);
		let err = res.expect_err("relation should reject object with both entity and value");
		let message = match err {
			Error::InvalidRequest { message } => message,
			_ => panic!("expected invalid request, got {err:?}"),
		};

		assert_eq!(
			message,
			"structured.relations[0].object must provide exactly one of entity or value."
		);
	}

	#[test]
	fn relation_rejects_valid_to_not_after_valid_from() {
		let structured = structured_relation(
			"alice",
			"met",
			StructuredRelationObject { entity: None, value: Some("bob".to_string()) },
			Some(OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("valid timestamp")),
			Some(OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("valid timestamp")),
		);
		let res = structured_fields::validate_structured_fields(
			&structured,
			"alice met bob",
			&serde_json::json!({
				"evidence": [{"quote": "alice met bob"}]
			}),
			None,
		);
		let err = res.expect_err("relation should require valid_to greater than valid_from");
		let message = match err {
			Error::InvalidRequest { message } => message,
			_ => panic!("expected invalid request, got {err:?}"),
		};

		assert_eq!(message, "structured.relations[0].valid_to must be greater than valid_from.");
	}

	#[test]
	fn relation_checks_subject_predicate_and_object_value_are_evidence_bound() {
		let subject_message = match structured_fields::validate_structured_fields(
			&structured_relation(
				"alice",
				"caused",
				StructuredRelationObject { entity: None, value: Some("outage".to_string()) },
				None,
				None,
			),
			"a critical outage was logged.",
			&serde_json::json!({"evidence": [{"quote": "caused an outage"}]}),
			None,
		) {
			Err(Error::InvalidRequest { message }) => message,
			res => panic!("expected invalid request, got {res:?}"),
		};

		assert!(
			subject_message.contains("structured.relations[0].subject.canonical is not supported")
		);

		let predicate_message = match structured_fields::validate_structured_fields(
			&structured_relation(
				"operator",
				"discovered",
				StructuredRelationObject { entity: None, value: Some("outage".to_string()) },
				None,
				None,
			),
			"operator monitored a system outage.",
			&serde_json::json!({"evidence": [{"quote": "operator saw outage"}]}),
			None,
		) {
			Err(Error::InvalidRequest { message }) => message,
			res => panic!("expected invalid request, got {res:?}"),
		};

		assert!(predicate_message.contains("structured.relations[0].predicate is not supported"));

		let object_message = match structured_fields::validate_structured_fields(
			&structured_relation(
				"operator",
				"noticed",
				StructuredRelationObject {
					entity: None,
					value: Some("service interruption".to_string()),
				},
				None,
				None,
			),
			"The operator noticed service latency during testing.",
			&serde_json::json!({"evidence": [{"quote": "The operator noticed service behavior"}]}),
			None,
		) {
			Err(Error::InvalidRequest { message }) => message,
			res => panic!("expected invalid request, got {res:?}"),
		};

		assert!(object_message.contains("structured.relations[0].object.value is not supported"));
	}

	#[test]
	fn relation_accepts_valid_structured_relation() {
		let structured = structured_relation(
			"alice",
			"works at",
			StructuredRelationObject {
				entity: Some(StructuredEntity {
					canonical: Some("acme corp".to_string()),
					kind: None,
					aliases: None,
				}),
				value: None,
			},
			Some(OffsetDateTime::from_unix_timestamp(1_699_900_000).expect("valid timestamp")),
			Some(OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("valid timestamp")),
		);
		let res = structured_fields::validate_structured_fields(
			&structured,
			"alice works at acme corp and reported progress.",
			&serde_json::json!({
				"evidence": [{"quote": "works at acme corp"}]
			}),
			None,
		);

		assert!(res.is_ok());
	}
}
