use sqlx::{Postgres, Transaction};

use super::{
	audit::record_ingest_decision,
	types::{AddEventContext, AddEventResult, ExtractedNote, NoteProcessingData},
	validation::{
		REJECT_STRUCTURED_INVALID, reject_extracted_note_if_evidence_invalid,
		reject_extracted_note_if_structured_invalid, reject_extracted_note_if_writegate_rejects,
	},
};
use crate::{NoteOp, Result, ingestion_profiles::IngestionProfileRef};
use elf_config::Config;
use elf_domain::{memory_policy::MemoryPolicyDecision, writegate::WritePolicyAudit};

#[allow(clippy::too_many_arguments)]
pub(super) async fn record_extracted_note_rejections(
	tx: &mut Transaction<'_, Postgres>,
	cfg: &Config,
	ctx: &AddEventContext<'_>,
	ingestion_profile: &IngestionProfileRef,
	note: &ExtractedNote,
	note_data: &NoteProcessingData,
	message_texts: &[String],
	message_policy_applied: &[bool],
	write_policy_audits: Option<&Vec<WritePolicyAudit>>,
) -> Result<Option<AddEventResult>> {
	if let Some(result) = reject_extracted_note_if_evidence_invalid(
		cfg,
		note.reason.as_ref(),
		&note_data.evidence,
		message_texts,
		message_policy_applied,
	) {
		let mut result = result;

		result.write_policy_audits = write_policy_audits.cloned();

		record_ingest_decision(
			tx,
			cfg,
			ctx,
			note,
			note_data.note_type.as_str(),
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
			Some(ingestion_profile.id.as_str()),
			Some(ingestion_profile.version),
			note_data.structured_present,
			note_data.graph_present,
			write_policy_audits.cloned(),
		)
		.await?;

		return Ok(Some(result));
	} else if let Some(result) = reject_extracted_note_if_structured_invalid(
		note_data.structured.as_ref(),
		note_data.text.as_str(),
		&note_data.evidence,
		note.reason.as_ref(),
	) {
		let mut result = result;

		result.write_policy_audits = write_policy_audits.cloned();

		record_ingest_decision(
			tx,
			cfg,
			ctx,
			note,
			note_data.note_type.as_str(),
			None,
			None,
			MemoryPolicyDecision::Reject,
			MemoryPolicyDecision::Reject,
			NoteOp::Rejected,
			Some(REJECT_STRUCTURED_INVALID),
			None,
			None,
			false,
			false,
			None,
			None,
			Some(ingestion_profile.id.as_str()),
			Some(ingestion_profile.version),
			note_data.structured_present,
			note_data.graph_present,
			write_policy_audits.cloned(),
		)
		.await?;

		return Ok(Some(result));
	} else if let Some(result) = reject_extracted_note_if_writegate_rejects(
		cfg,
		note.reason.as_ref(),
		note_data.note_type.as_str(),
		note_data.scope.as_str(),
		note_data.text.as_str(),
	) {
		let mut result = result;

		result.write_policy_audits = write_policy_audits.cloned();

		record_ingest_decision(
			tx,
			cfg,
			ctx,
			note,
			note_data.note_type.as_str(),
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
			Some(ingestion_profile.id.as_str()),
			Some(ingestion_profile.version),
			note_data.structured_present,
			note_data.graph_present,
			write_policy_audits.cloned(),
		)
		.await?;

		return Ok(Some(result));
	}

	Ok(None)
}
