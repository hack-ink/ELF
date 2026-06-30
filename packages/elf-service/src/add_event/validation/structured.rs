use crate::{
	Error, NoteOp,
	add_event::{
		types::{AddEventResult, EvidenceQuote},
		validation::REJECT_STRUCTURED_INVALID,
	},
	structured_fields::{self, StructuredFields},
};
use elf_domain::memory_policy::MemoryPolicyDecision;

pub(in crate::add_event) fn reject_extracted_note_if_structured_invalid(
	structured: Option<&StructuredFields>,
	text: &str,
	evidence: &[EvidenceQuote],
	reason: Option<&String>,
) -> Option<AddEventResult> {
	let structured = structured?;

	if structured.is_effectively_empty() {
		return None;
	}

	let event_evidence: Vec<(usize, String)> =
		evidence.iter().map(|q| (q.message_index, q.quote.clone())).collect();

	if let Err(err) = structured_fields::validate_structured_fields(
		structured,
		text,
		&serde_json::json!({}),
		Some(event_evidence.as_slice()),
	) {
		tracing::info!(error = %err, "Rejecting extracted note due to invalid structured fields.");

		let field_path = extract_structured_rejection_field_path(&err);

		return Some(AddEventResult {
			note_id: None,
			op: NoteOp::Rejected,
			policy_decision: MemoryPolicyDecision::Reject,
			reason_code: Some(REJECT_STRUCTURED_INVALID.to_string()),
			reason: reason.cloned(),
			field_path,
			write_policy_audits: None,
		});
	}

	None
}

fn extract_structured_rejection_field_path(err: &Error) -> Option<String> {
	match err {
		Error::NonEnglishInput { field } => Some(field.clone()),
		Error::InvalidRequest { message } if message.starts_with("structured.") =>
			message.split_whitespace().next().map(ToString::to_string),
		_ => None,
	}
}
