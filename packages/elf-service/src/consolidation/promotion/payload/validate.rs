use crate::{Error, Result, consolidation::types::PromotedMemoryPayload};
use elf_config::Config;
use elf_domain::writegate::{self, NoteInput};

pub(in crate::consolidation) fn validate_promoted_memory_payload(
	payload: &PromotedMemoryPayload,
	effective_scope: &str,
	cfg: &Config,
) -> Result<()> {
	let gate = NoteInput {
		note_type: payload.note_type.clone(),
		scope: effective_scope.to_string(),
		text: payload.text.clone(),
	};

	if let Err(code) = writegate::writegate(&gate, cfg) {
		return Err(Error::InvalidRequest {
			message: format!(
				"proposed memory failed writegate: {}",
				crate::writegate_reason_code(code)
			),
		});
	}

	Ok(())
}
