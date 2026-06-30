use time::OffsetDateTime;

use crate::{
	ElfService, Result,
	access::ORG_PROJECT_ID,
	add_event::{
		rejection,
		types::{
			AddEventContext, AddEventRequest, AddEventResult, ExtractedNote, NoteProcessingData,
		},
	},
	ingestion_profiles::IngestionProfileRef,
};
use elf_domain::writegate::WritePolicyAudit;

impl ElfService {
	#[allow(clippy::too_many_arguments)]
	pub(in crate::add_event) async fn process_extracted_note(
		&self,
		req: &AddEventRequest,
		ingestion_profile: &IngestionProfileRef,
		message_texts: &[String],
		message_policy_applied: &[bool],
		write_policy_audits: Option<&Vec<WritePolicyAudit>>,
		note: ExtractedNote,
		now: OffsetDateTime,
		embed_version: &str,
		dry_run: bool,
	) -> Result<AddEventResult> {
		let note_data = NoteProcessingData::from_request_and_note(req, &note);
		let effective_project_id = if note_data.scope.trim() == "org_shared" {
			ORG_PROJECT_ID
		} else {
			req.project_id.as_str()
		};
		let ctx = AddEventContext {
			tenant_id: req.tenant_id.as_str(),
			project_id: effective_project_id,
			agent_id: req.agent_id.as_str(),
			scope: note_data.scope.as_str(),
			now,
		};
		let mut tx = self.db.pool.begin().await?;

		if let Some(result) = rejection::record_extracted_note_rejections(
			&mut tx,
			&self.cfg,
			&ctx,
			ingestion_profile,
			&note,
			&note_data,
			message_texts,
			message_policy_applied,
			write_policy_audits,
		)
		.await?
		{
			tx.commit().await?;

			return Ok(result);
		}

		let result = self
			.apply_extracted_note_decision(
				req,
				ingestion_profile,
				&mut tx,
				&ctx,
				&note,
				&note_data,
				note_data.note_type.as_str(),
				effective_project_id,
				now,
				embed_version,
				dry_run,
				write_policy_audits,
			)
			.await?;

		tx.commit().await?;

		Ok(result)
	}
}
