mod non_english;

use crate::{
	Error, NoteOp, Result,
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
		if let Some(path) = non_english::find_non_english_path_in_structured(
			note.structured.as_ref(),
			&format!("$.notes[{idx}].structured"),
		) {
			return Err(Error::NonEnglishInput { field: path });
		}
		if let Some(path) = non_english::find_non_english_path(
			&note.source_ref,
			&format!("$.notes[{idx}].source_ref"),
		) {
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

fn extract_structured_rejection_field_path(err: &Error) -> Option<String> {
	match err {
		Error::NonEnglishInput { field } => Some(field.clone()),
		Error::InvalidRequest { message } if message.starts_with("structured.") =>
			message.split_whitespace().next().map(ToString::to_string),
		_ => None,
	}
}
