use serde::Deserialize;
use serde_json::Value;

use crate::{
	Error, Result,
	structured_fields::types::{StructuredEntity, StructuredFields, StructuredRelation},
};
use elf_domain::{english_gate, evidence};

const MAX_LIST_ITEMS: usize = 64;
const MAX_ENTITIES: usize = 32;
const MAX_RELATIONS: usize = 64;
const MAX_ALIASES: usize = 16;
const MAX_ITEM_CHARS: usize = 1_000;

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
