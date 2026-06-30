mod bounds;
mod entity;
mod quotes;
mod relation;
mod text;

use serde_json::Value;

use crate::{
	Error, Result,
	structured_fields::{
		types::StructuredFields,
		validation::bounds::{MAX_ENTITIES, MAX_RELATIONS},
	},
};

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
		quotes::extract_source_ref_quotes(source_ref)
	};

	if let Some(summary) = structured.summary.as_ref() {
		text::validate_text_field(summary, "structured.summary")?;
	}
	if let Some(entities) = structured.entities.as_ref() {
		bounds::validate_list_field_count(entities.len(), MAX_ENTITIES, "structured.entities")?;

		for (idx, entity) in entities.iter().enumerate() {
			let base = format!("structured.entities[{idx}]");

			entity::validate_structured_entity(entity, &base, true)?;
		}
	}
	if let Some(relations) = structured.relations.as_ref() {
		bounds::validate_list_field_count(relations.len(), MAX_RELATIONS, "structured.relations")?;

		for (idx, relation) in relations.iter().enumerate() {
			relation::validate_structured_relation(
				relation,
				note_text,
				&evidence_quotes,
				&format!("structured.relations[{idx}]"),
			)?;
		}
	}
	if let Some(facts) = structured.facts.as_ref() {
		bounds::validate_list_field(facts, "structured.facts")?;

		for (idx, fact) in facts.iter().enumerate() {
			text::validate_text_field(fact, &format!("structured.facts[{idx}]"))?;

			if !quotes::fact_is_evidence_bound(fact, note_text, &evidence_quotes) {
				return Err(Error::InvalidRequest {
					message: format!(
						"structured.facts[{idx}] is not supported by note text or evidence quotes."
					),
				});
			}
		}
	}
	if let Some(concepts) = structured.concepts.as_ref() {
		bounds::validate_list_field(concepts, "structured.concepts")?;

		for (idx, concept) in concepts.iter().enumerate() {
			text::validate_text_field(concept, &format!("structured.concepts[{idx}]"))?;
		}
	}

	Ok(())
}

/// Validates event-evidence quotes against their source messages.
pub fn event_evidence_quotes(messages: &[String], evidence: &[(usize, String)]) -> Result<()> {
	quotes::event_evidence_quotes(messages, evidence)
}
