use sqlx::{PgConnection, Postgres, Transaction};
use time::OffsetDateTime;

use crate::{
	ElfService, ResolveUpdateArgs, Result, UpdateDecision,
	access::ORG_PROJECT_ID,
	add_event::{
		audit, materialize,
		policy::{self},
		types::{
			AddEventContext, AddEventRequest, AddEventResult, ExtractedNote, NoteProcessingData,
			PersistExtractedNoteArgs,
		},
	},
	ingestion_profiles::IngestionProfileRef,
};
use elf_domain::{memory_policy::MemoryPolicyDecision, ttl, writegate::WritePolicyAudit};

impl ElfService {
	#[allow(clippy::too_many_arguments)]
	pub(in crate::add_event) async fn apply_extracted_note_decision(
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
