use crate::{NoteOp, add_event::types::AddEventResult};
use elf_config::Config;
use elf_domain::{
	memory_policy::MemoryPolicyDecision,
	writegate::{self, NoteInput},
};

pub(in crate::add_event) fn reject_extracted_note_if_writegate_rejects(
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
