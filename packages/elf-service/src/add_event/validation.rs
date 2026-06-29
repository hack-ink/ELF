use crate::{
	Error, NoteOp, REJECT_EVIDENCE_MISMATCH, REJECT_WRITE_POLICY_MISMATCH, Result,
	add_event::types::{
		AddEventRequest, AddEventResult, EventMessage, EvidenceQuote, ProcessedEventOutput,
	},
	structured_fields::{self, StructuredFields},
};
use elf_config::Config;
use elf_domain::{
	english_gate, evidence,
	memory_policy::MemoryPolicyDecision,
	writegate::{self, NoteInput, WritePolicyAudit, WritePolicyError},
};

pub(super) const REJECT_STRUCTURED_INVALID: &str = "REJECT_STRUCTURED_INVALID";

pub(super) fn validate_add_event_request(req: &AddEventRequest) -> Result<()> {
	if req.messages.is_empty() {
		return Err(Error::InvalidRequest { message: "Messages list is empty.".to_string() });
	}
	if req.tenant_id.trim().is_empty()
		|| req.project_id.trim().is_empty()
		|| req.agent_id.trim().is_empty()
	{
		return Err(Error::InvalidRequest {
			message: "tenant_id, project_id, and agent_id are required.".to_string(),
		});
	}

	if let Some(scope) = req.scope.as_ref()
		&& scope.trim().is_empty()
	{
		return Err(Error::InvalidRequest {
			message: "scope must not be empty when provided.".to_string(),
		});
	}
	if let Some(profile) = req.ingestion_profile.as_ref() {
		if profile.id.trim().is_empty() {
			return Err(Error::InvalidRequest {
				message: "ingestion_profile.id must not be empty.".to_string(),
			});
		}

		if let Some(version) = profile.version
			&& version <= 0
		{
			return Err(Error::InvalidRequest {
				message: "ingestion_profile.version must be greater than zero.".to_string(),
			});
		}
	}

	for (idx, msg) in req.messages.iter().enumerate() {
		if !english_gate::is_english_natural_language(msg.content.as_str()) {
			return Err(Error::NonEnglishInput { field: format!("$.messages[{idx}].content") });
		}
	}

	Ok(())
}

pub(super) fn apply_write_policies_to_messages(
	messages: &[EventMessage],
) -> Result<ProcessedEventOutput> {
	let mut message_policy_applied = Vec::with_capacity(messages.len());
	let mut write_policy_audits = Vec::new();
	let mut transformed_messages = Vec::with_capacity(messages.len());

	for message in messages {
		let (transformed_message, audit) = apply_write_policy_to_message(message)?;

		message_policy_applied.push(audit.is_some());

		if let Some(audit) = audit {
			write_policy_audits.push(audit);
		}

		transformed_messages.push(transformed_message);
	}

	Ok((
		transformed_messages,
		message_policy_applied,
		if write_policy_audits.is_empty() { None } else { Some(write_policy_audits) },
	))
}

pub(super) fn apply_write_policy_to_message(
	message: &EventMessage,
) -> Result<(EventMessage, Option<WritePolicyAudit>)> {
	let result =
		writegate::apply_write_policy(message.content.as_str(), message.write_policy.as_ref())
			.map_err(|err| {
				let message = match err {
					WritePolicyError::InvalidSpan => "Invalid write_policy span provided.",
					WritePolicyError::OverlappingOps => "Overlapping write_policy spans provided.",
				};

				Error::InvalidRequest { message: message.to_string() }
			})?;
	let has_policy = message.write_policy.is_some();
	let mut transformed = message.clone();

	transformed.content = result.transformed;

	Ok((transformed, if has_policy { Some(result.audit) } else { None }))
}

pub(super) fn reject_extracted_note_if_evidence_invalid(
	cfg: &Config,
	reason: Option<&String>,
	evidence: &[EvidenceQuote],
	message_texts: &[String],
	message_policy_applied: &[bool],
) -> Option<AddEventResult> {
	if evidence.is_empty()
		|| evidence.len() < cfg.security.evidence_min_quotes as usize
		|| evidence.len() > cfg.security.evidence_max_quotes as usize
	{
		return Some(AddEventResult {
			note_id: None,
			op: NoteOp::Rejected,
			policy_decision: MemoryPolicyDecision::Reject,
			reason_code: Some(REJECT_EVIDENCE_MISMATCH.to_string()),
			reason: reason.cloned(),
			field_path: None,
			write_policy_audits: None,
		});
	}

	for quote in evidence {
		if quote.quote.len() > cfg.security.evidence_max_quote_chars as usize {
			return Some(AddEventResult {
				note_id: None,
				op: NoteOp::Rejected,
				policy_decision: MemoryPolicyDecision::Reject,
				reason_code: Some(REJECT_EVIDENCE_MISMATCH.to_string()),
				reason: reason.cloned(),
				field_path: None,
				write_policy_audits: None,
			});
		}
		if !evidence::evidence_matches(message_texts, quote.message_index, quote.quote.as_str()) {
			let reason_code =
				message_policy_applied.get(quote.message_index).is_some_and(|applied| *applied);

			return Some(AddEventResult {
				note_id: None,
				op: NoteOp::Rejected,
				policy_decision: MemoryPolicyDecision::Reject,
				reason_code: Some(if reason_code {
					REJECT_WRITE_POLICY_MISMATCH.to_string()
				} else {
					REJECT_EVIDENCE_MISMATCH.to_string()
				}),
				reason: reason.cloned(),
				field_path: None,
				write_policy_audits: None,
			});
		}
	}

	None
}

pub(super) fn reject_extracted_note_if_structured_invalid(
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

pub(super) fn reject_extracted_note_if_writegate_rejects(
	cfg: &Config,
	reason: Option<&String>,
	note_type: &str,
	scope: &str,
	text: &str,
) -> Option<AddEventResult> {
	let gate_input = NoteInput {
		note_type: note_type.to_string(),
		scope: scope.to_string(),
		text: text.to_string(),
	};

	if let Err(code) = writegate::writegate(&gate_input, cfg) {
		return Some(AddEventResult {
			note_id: None,
			op: NoteOp::Rejected,
			policy_decision: MemoryPolicyDecision::Reject,
			reason_code: Some(crate::writegate_reason_code(code).to_string()),
			reason: reason.cloned(),
			field_path: None,
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
