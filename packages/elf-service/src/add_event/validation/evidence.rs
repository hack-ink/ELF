use crate::{
	NoteOp, REJECT_EVIDENCE_MISMATCH, REJECT_WRITE_POLICY_MISMATCH,
	add_event::types::{AddEventResult, EvidenceQuote},
};
use elf_config::Config;
use elf_domain::{evidence, memory_policy::MemoryPolicyDecision};

pub(in crate::add_event) fn reject_extracted_note_if_evidence_invalid(
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
