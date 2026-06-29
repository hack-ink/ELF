use serde_json::Value;

use crate::{
	Error, NoteOp, Result, StructuredFields,
	add_note::types::{self, AddNoteInput, AddNoteRequest, AddNoteResult},
	structured_fields,
};
use elf_config::Config;
use elf_domain::{
	english_gate,
	memory_policy::MemoryPolicyDecision,
	writegate::{self, NoteInput, WritePolicy, WritePolicyAudit, WritePolicyError},
};

const REJECT_STRUCTURED_INVALID: &str = "REJECT_STRUCTURED_INVALID";

pub(super) fn normalize_add_note_request(mut req: AddNoteRequest) -> AddNoteRequest {
	for note in &mut req.notes {
		if note.source_ref.is_null() {
			note.source_ref = types::default_source_ref();
		}
	}

	req
}

pub(super) fn validate_add_note_request(req: &AddNoteRequest) -> Result<()> {
	if req.notes.is_empty() {
		return Err(Error::InvalidRequest { message: "Notes list is empty.".to_string() });
	}
	if req.tenant_id.trim().is_empty()
		|| req.project_id.trim().is_empty()
		|| req.agent_id.trim().is_empty()
		|| req.scope.trim().is_empty()
	{
		return Err(Error::InvalidRequest {
			message: "tenant_id, project_id, agent_id, and scope are required.".to_string(),
		});
	}

	for (idx, note) in req.notes.iter().enumerate() {
		if !note.source_ref.is_object() {
			return Err(Error::InvalidRequest {
				message: "source_ref must be a JSON object.".to_string(),
			});
		}
		if !english_gate::is_english_natural_language(note.text.as_str()) {
			return Err(Error::NonEnglishInput { field: format!("$.notes[{idx}].text") });
		}

		if let Some(key) = note.key.as_ref()
			&& !english_gate::is_english_identifier(key)
		{
			return Err(Error::NonEnglishInput { field: format!("$.notes[{idx}].key") });
		}
		if let Some(path) = find_non_english_path_in_structured(
			note.structured.as_ref(),
			&format!("$.notes[{idx}].structured"),
		) {
			return Err(Error::NonEnglishInput { field: path });
		}
		if let Some(path) =
			find_non_english_path(&note.source_ref, &format!("$.notes[{idx}].source_ref"))
		{
			return Err(Error::NonEnglishInput { field: path });
		}
	}

	Ok(())
}

pub(super) fn reject_note_if_structured_invalid(note: &AddNoteInput) -> Option<AddNoteResult> {
	if let Some(structured) = note.structured.as_ref()
		&& let Err(err) = structured_fields::validate_structured_fields(
			structured,
			note.text.as_str(),
			&note.source_ref,
			None,
		) {
		tracing::info!(error = %err, "Rejecting note due to invalid structured fields.");

		let field_path = extract_structured_rejection_field_path(&err);

		return Some(AddNoteResult {
			note_id: None,
			op: NoteOp::Rejected,
			policy_decision: MemoryPolicyDecision::Reject,
			reason_code: Some(REJECT_STRUCTURED_INVALID.to_string()),
			field_path,
			write_policy_audit: None,
		});
	}

	None
}

pub(super) fn reject_note_if_writegate_rejects(
	cfg: &Config,
	scope: &str,
	note: &AddNoteInput,
) -> Option<AddNoteResult> {
	let gate_input = NoteInput {
		note_type: note.r#type.clone(),
		scope: scope.to_string(),
		text: note.text.clone(),
	};

	if let Err(code) = writegate::writegate(&gate_input, cfg) {
		return Some(AddNoteResult {
			note_id: None,
			op: NoteOp::Rejected,
			policy_decision: MemoryPolicyDecision::Reject,
			reason_code: Some(crate::writegate_reason_code(code).to_string()),
			field_path: None,
			write_policy_audit: None,
		});
	}

	None
}

pub(super) fn apply_write_policy_to_note(
	policy: Option<&WritePolicy>,
	text: &str,
) -> Result<(String, Option<WritePolicyAudit>)> {
	let result = writegate::apply_write_policy(text, policy).map_err(|err| {
		let message = match err {
			WritePolicyError::InvalidSpan => "Invalid write_policy span provided.",
			WritePolicyError::OverlappingOps => "Overlapping write_policy spans provided.",
		};

		Error::InvalidRequest { message: message.to_string() }
	})?;

	Ok((result.transformed, policy.is_some().then_some(result.audit)))
}

fn find_non_english_path_in_structured(
	structured: Option<&StructuredFields>,
	base: &str,
) -> Option<String> {
	let structured = structured?;

	if let Some(summary) = structured.summary.as_ref()
		&& !english_gate::is_english_natural_language(summary)
	{
		return Some(format!("{base}.summary"));
	}
	if let Some(items) = structured.facts.as_ref() {
		for (idx, item) in items.iter().enumerate() {
			if !english_gate::is_english_natural_language(item) {
				return Some(format!("{base}.facts[{idx}]"));
			}
		}
	}
	if let Some(items) = structured.concepts.as_ref() {
		for (idx, item) in items.iter().enumerate() {
			if !english_gate::is_english_natural_language(item) {
				return Some(format!("{base}.concepts[{idx}]"));
			}
		}
	}
	if let Some(items) = structured.entities.as_ref() {
		for (idx, entity) in items.iter().enumerate() {
			let base = format!("{base}.entities[{idx}]");

			if let Some(canonical) = entity.canonical.as_ref()
				&& !english_gate::is_english_natural_language(canonical)
			{
				return Some(format!("{base}.canonical"));
			}
			if let Some(kind) = entity.kind.as_ref()
				&& !english_gate::is_english_natural_language(kind)
			{
				return Some(format!("{base}.kind"));
			}
			if let Some(aliases) = entity.aliases.as_ref() {
				for (alias_idx, alias) in aliases.iter().enumerate() {
					if !english_gate::is_english_natural_language(alias) {
						return Some(format!("{base}.aliases[{alias_idx}]"));
					}
				}
			}
		}
	}
	if let Some(items) = structured.relations.as_ref() {
		for (idx, relation) in items.iter().enumerate() {
			let base = format!("{base}.relations[{idx}]");

			if let Some(subject) = relation.subject.as_ref() {
				let subject_base = format!("{base}.subject");

				if let Some(canonical) = subject.canonical.as_ref()
					&& !english_gate::is_english_natural_language(canonical)
				{
					return Some(format!("{subject_base}.canonical"));
				}
				if let Some(kind) = subject.kind.as_ref()
					&& !english_gate::is_english_natural_language(kind)
				{
					return Some(format!("{subject_base}.kind"));
				}
				if let Some(aliases) = subject.aliases.as_ref() {
					for (alias_idx, alias) in aliases.iter().enumerate() {
						if !english_gate::is_english_natural_language(alias) {
							return Some(format!("{subject_base}.aliases[{alias_idx}]"));
						}
					}
				}
			}
			if let Some(predicate) = relation.predicate.as_ref()
				&& !english_gate::is_english_natural_language(predicate)
			{
				return Some(format!("{base}.predicate"));
			}
			if let Some(object) = relation.object.as_ref() {
				if let Some(entity) = object.entity.as_ref() {
					let object_base = format!("{base}.object.entity");

					if let Some(canonical) = entity.canonical.as_ref()
						&& !english_gate::is_english_natural_language(canonical)
					{
						return Some(format!("{object_base}.canonical"));
					}
					if let Some(kind) = entity.kind.as_ref()
						&& !english_gate::is_english_natural_language(kind)
					{
						return Some(format!("{object_base}.kind"));
					}
					if let Some(aliases) = entity.aliases.as_ref() {
						for (alias_idx, alias) in aliases.iter().enumerate() {
							if !english_gate::is_english_natural_language(alias) {
								return Some(format!("{object_base}.aliases[{alias_idx}]"));
							}
						}
					}
				}
				if let Some(value) = object.value.as_ref()
					&& !english_gate::is_english_natural_language(value)
				{
					return Some(format!("{base}.object.value"));
				}
			}
		}
	}

	None
}

fn find_non_english_path(value: &Value, path: &str) -> Option<String> {
	find_non_english_path_inner(value, path, true)
}

fn find_non_english_path_inner(
	value: &Value,
	path: &str,
	is_identifier_lane: bool,
) -> Option<String> {
	fn has_english_gate(text: &str, is_identifier_lane: bool) -> bool {
		if is_identifier_lane {
			return english_gate::is_english_identifier(text);
		}

		english_gate::is_english_natural_language(text)
	}

	match value {
		Value::String(text) =>
			if !has_english_gate(text, is_identifier_lane) {
				Some(path.to_string())
			} else {
				None
			},
		Value::Array(items) => {
			for (idx, item) in items.iter().enumerate() {
				let child_path = format!("{path}[{idx}]");

				if let Some(found) =
					find_non_english_path_inner(item, &child_path, is_identifier_lane)
				{
					return Some(found);
				}
			}

			None
		},
		Value::Object(map) => {
			for (key, value) in map.iter() {
				let identifier_lane = is_identifier_lane
					|| matches!(key.as_str(), "ref" | "schema" | "resolver" | "hashes" | "state");
				let child_path = format!("{path}[\"{}\"]", escape_json_path_key(key));

				if let Some(found) =
					find_non_english_path_inner(value, &child_path, identifier_lane)
				{
					return Some(found);
				}
			}

			None
		},
		_ => None,
	}
}

fn escape_json_path_key(key: &str) -> String {
	key.replace('\\', "\\\\").replace('"', "\\\"")
}

fn extract_structured_rejection_field_path(err: &Error) -> Option<String> {
	match err {
		Error::NonEnglishInput { field } => Some(field.clone()),
		Error::InvalidRequest { message } if message.starts_with("structured.") =>
			message.split_whitespace().next().map(ToString::to_string),
		_ => None,
	}
}
