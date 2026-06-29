use sqlx::{Postgres, Transaction};

use crate::{
	NoteOp, Result,
	add_note::{
		audit,
		types::{AddNoteContext, AddNoteInput, AddNoteResult},
		validation::{self},
	},
};
use elf_config::Config;
use elf_domain::{memory_policy::MemoryPolicyDecision, writegate::WritePolicyAudit};

pub(super) async fn handle_rejection_paths(
	tx: &mut Transaction<'_, Postgres>,
	cfg: &Config,
	ctx: &AddNoteContext<'_>,
	note: &AddNoteInput,
	write_policy_audit: Option<&WritePolicyAudit>,
) -> Result<Option<AddNoteResult>> {
	if let Some(result) = validation::reject_note_if_structured_invalid(note) {
		let mut result = result;

		result.write_policy_audit = write_policy_audit.cloned();

		audit::record_ingest_decision(
			tx,
			cfg,
			ctx,
			note,
			None,
			None,
			MemoryPolicyDecision::Reject,
			MemoryPolicyDecision::Reject,
			NoteOp::Rejected,
			result.reason_code.as_deref(),
			None,
			None,
			false,
			false,
			None,
			None,
			write_policy_audit.cloned(),
		)
		.await?;

		return Ok(Some(result));
	}
	if let Some(result) = validation::reject_note_if_writegate_rejects(cfg, ctx.scope, note) {
		let mut result = result;

		result.write_policy_audit = write_policy_audit.cloned();

		audit::record_ingest_decision(
			tx,
			cfg,
			ctx,
			note,
			None,
			None,
			MemoryPolicyDecision::Reject,
			MemoryPolicyDecision::Reject,
			NoteOp::Rejected,
			result.reason_code.as_deref(),
			None,
			None,
			false,
			false,
			None,
			None,
			write_policy_audit.cloned(),
		)
		.await?;

		return Ok(Some(result));
	}

	Ok(None)
}
