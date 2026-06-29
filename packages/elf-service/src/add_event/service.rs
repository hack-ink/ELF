use serde_json;
use sqlx::{PgConnection, Postgres, Transaction};
use time::{Duration, OffsetDateTime};

use crate::{
	ElfService, Error, ResolveUpdateArgs, Result, UpdateDecision,
	access::ORG_PROJECT_ID,
	add_event::{
		audit, materialize,
		policy::{self},
		rejection,
		types::{
			AddEventContext, AddEventRequest, AddEventResponse, AddEventResult, ExtractedNote,
			ExtractorOutput, NoteProcessingData, PersistExtractedNoteArgs,
		},
		validation::{self},
	},
	ingestion_profiles::{self, IngestionProfileRef},
};
use elf_domain::{memory_policy::MemoryPolicyDecision, ttl, writegate::WritePolicyAudit};

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

	#[allow(clippy::too_many_arguments)]
	async fn process_extracted_note(
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

	#[allow(clippy::too_many_arguments)]
	async fn apply_extracted_note_decision(
		&self,
		req: &AddEventRequest,
		ingestion_profile: &IngestionProfileRef,
		tx: &mut Transaction<'_, Postgres>,
		ctx: &AddEventContext<'_>,
		note: &ExtractedNote,
		note_data: &NoteProcessingData,
		note_type: &str,
		project_id: &str,
		now: OffsetDateTime,
		embed_version: &str,
		dry_run: bool,
		write_policy_audits: Option<&Vec<WritePolicyAudit>>,
	) -> Result<AddEventResult> {
		let decision = self.resolve_extracted_note_update(note, req, note_data, tx, now).await?;
		let metadata = decision.metadata();
		let base_decision = policy::base_decision_for_update(
			&decision,
			note_data.structured_present,
			note_data.graph_present,
		);
		let (policy_decision, decision_policy_rule, min_confidence, min_importance) =
			policy::resolve_policy_for_update(&self.cfg, note_data, base_decision);
		let ignore_reason_code = policy::ignore_reason_code_for_policy(
			base_decision,
			policy_decision,
			metadata.matched_dup,
		);
		let should_apply = matches!(
			policy_decision,
			MemoryPolicyDecision::Remember | MemoryPolicyDecision::Update
		);
		let mut result = policy::build_result_from_decision(
			&decision,
			policy_decision,
			note_data.reason.clone(),
			note_data.structured_present || note_data.graph_present,
		);

		policy::apply_policy_ignore_adjustments(
			&mut result,
			&decision,
			policy_decision,
			ignore_reason_code,
		);

		let mut note_version_id = None;

		if should_apply && !dry_run {
			let persist_args = PersistExtractedNoteArgs {
				req,
				project_id,
				structured: note_data.structured.as_ref(),
				key: note.key.as_deref(),
				reason: note.reason.as_ref(),
				note_type,
				text: note_data.text.as_str(),
				scope: note_data.scope.as_str(),
				importance: note_data.importance,
				confidence: note_data.confidence,
				expires_at: ttl::compute_expires_at(
					note_data.ttl_days,
					note_data.note_type.as_str(),
					&self.cfg,
					now,
				),
				source_ref: serde_json::json!({
					"evidence": note_data.evidence.clone(),
					"reason": note_data.reason.clone().unwrap_or_default(),
					"ingestion_profile": serde_json::json!({
						"id": ingestion_profile.id,
						"version": ingestion_profile.version,
					}),
				}),
				now,
				embed_version,
			};
			let persisted = materialize::persist_extracted_note_decision(
				tx,
				persist_args,
				decision,
				policy_decision,
			)
			.await?;

			result = persisted.0;
			note_version_id = persisted.1;
		}

		result.write_policy_audits = write_policy_audits.cloned();

		audit::record_ingest_decision(
			tx,
			&self.cfg,
			ctx,
			note,
			note_data.note_type.as_str(),
			result.note_id,
			note_version_id,
			base_decision,
			policy_decision,
			result.op,
			result.reason_code.as_deref(),
			decision_policy_rule.as_deref(),
			metadata.similarity_best,
			metadata.key_match,
			metadata.matched_dup,
			min_confidence,
			min_importance,
			Some(ingestion_profile.id.as_str()),
			Some(ingestion_profile.version),
			note_data.structured_present,
			note_data.graph_present,
			write_policy_audits.cloned(),
		)
		.await?;

		Ok(result)
	}

	async fn resolve_extracted_note_update(
		&self,
		note: &ExtractedNote,
		req: &AddEventRequest,
		note_data: &NoteProcessingData,
		tx: &mut PgConnection,
		now: OffsetDateTime,
	) -> Result<UpdateDecision> {
		crate::resolve_update(
			tx,
			ResolveUpdateArgs {
				cfg: &self.cfg,
				providers: &self.providers,
				tenant_id: req.tenant_id.as_str(),
				project_id: if note_data.scope.trim() == "org_shared" {
					ORG_PROJECT_ID
				} else {
					req.project_id.as_str()
				},
				agent_id: req.agent_id.as_str(),
				scope: note_data.scope.as_str(),
				note_type: note_data.note_type.as_str(),
				key: note.key.as_deref(),
				text: note_data.text.as_str(),
				now,
			},
		)
		.await
	}
}
