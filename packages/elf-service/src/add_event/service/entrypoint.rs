use time::{Duration, OffsetDateTime};

use crate::{
	ElfService, Error, Result,
	add_event::{
		types::{AddEventRequest, AddEventResponse, ExtractorOutput},
		validation,
	},
	ingestion_profiles,
};

impl ElfService {
	/// Extracts notes from an event transcript and optionally persists the accepted results.
	pub async fn add_event(&self, req: AddEventRequest) -> Result<AddEventResponse> {
		validation::validate_add_event_request(&req)?;

		let resolved_profile = ingestion_profiles::resolve_add_event_profile(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.ingestion_profile.as_ref(),
		)
		.await?;
		let (messages, message_policy_applied, write_policy_audits) =
			validation::apply_write_policies_to_messages(req.messages.as_slice())?;
		let message_texts: Vec<String> =
			messages.iter().map(|message| message.content.clone()).collect();
		let messages_json =
			serde_json::to_string(&messages).map_err(|_| Error::InvalidRequest {
				message: "Failed to serialize messages for extractor.".to_string(),
			})?;
		let extractor_messages = resolved_profile.build_extractor_messages(
			&messages_json,
			self.cfg.memory.max_notes_per_add_event,
			self.cfg.memory.max_note_chars,
		)?;
		let llm_cfg = resolved_profile.resolved_llm_config(&self.cfg.providers.llm_extractor);
		let extracted_raw = self.providers.extractor.extract(&llm_cfg, &extractor_messages).await?;
		let max_notes = self.cfg.memory.max_notes_per_add_event as usize;
		let mut extracted: ExtractorOutput = serde_json::from_value(extracted_raw.clone())
			.map_err(|_| Error::InvalidRequest {
				message: "Extractor output is missing notes array.".to_string(),
			})?;

		if extracted.notes.len() > max_notes {
			extracted.notes.truncate(max_notes);
		}

		let extracted_json = serde_json::to_value(&extracted).map_err(|_| {
			Error::InvalidRequest { message: "Failed to serialize extracted notes.".to_string() }
		})?;
		let base_now = OffsetDateTime::now_utc();
		let embed_version = crate::embedding_version(&self.cfg);
		let dry_run = req.dry_run.unwrap_or(false);
		let mut results = Vec::with_capacity(extracted.notes.len());

		for (note_idx, note) in extracted.notes.into_iter().enumerate() {
			let now = base_now + Duration::microseconds(note_idx as i64);

			results.push(
				self.process_extracted_note(
					&req,
					&resolved_profile.profile_ref,
					&message_texts,
					&message_policy_applied,
					write_policy_audits.as_ref(),
					note,
					now,
					embed_version.as_str(),
					dry_run,
				)
				.await?,
			);
		}

		Ok(AddEventResponse {
			extracted: extracted_json,
			results,
			ingestion_profile: Some(resolved_profile.profile_ref),
		})
	}
}
