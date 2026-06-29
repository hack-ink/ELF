use sqlx::{Postgres, Transaction};
use time::{Duration, OffsetDateTime};

use crate::{
	ElfService, ResolveUpdateArgs, Result, UpdateDecision, UpdateDecisionMetadata,
	access::ORG_PROJECT_ID,
	add_note::{
		audit,
		policy::{self},
		rejection,
		types::{AddNoteContext, AddNoteInput, AddNoteRequest, AddNoteResponse, AddNoteResult},
		validation::{self},
	},
};

impl ElfService {
	/// Validates and persists notes supplied directly by the caller.
	pub async fn add_note(&self, req: AddNoteRequest) -> Result<AddNoteResponse> {
		let req = validation::normalize_add_note_request(req);

		validation::validate_add_note_request(&req)?;

		let base_now = OffsetDateTime::now_utc();
		let embed_version = crate::embedding_version(&self.cfg);
		let AddNoteRequest { tenant_id, project_id, agent_id, scope, notes } = req;
		let effective_project_id =
			if scope.trim() == "org_shared" { ORG_PROJECT_ID } else { project_id.as_str() };
		let mut results = Vec::with_capacity(notes.len());

		for (note_idx, note) in notes.into_iter().enumerate() {
			let now = base_now + Duration::microseconds(note_idx as i64);
			let ctx = AddNoteContext {
				tenant_id: tenant_id.as_str(),
				project_id: effective_project_id,
				agent_id: agent_id.as_str(),
				scope: scope.as_str(),
				now,
				embed_version: embed_version.as_str(),
			};

			results.push(self.process_add_note_input(&ctx, note).await?);
		}

		Ok(AddNoteResponse { results })
	}

	async fn process_add_note_input(
		&self,
		ctx: &AddNoteContext<'_>,
		note: AddNoteInput,
	) -> Result<AddNoteResult> {
		let mut note = note;
		let (transformed, write_policy_audit) =
			validation::apply_write_policy_to_note(note.write_policy.as_ref(), note.text.as_str())?;

		note.text = transformed;

		let (structured_present, graph_present) =
			policy::structured_and_graph_present(note.structured.as_ref());
		let mut tx = self.db.pool.begin().await?;

		if let Some(result) = rejection::handle_rejection_paths(
			&mut tx,
			&self.cfg,
			ctx,
			&note,
			write_policy_audit.as_ref(),
		)
		.await?
		{
			tx.commit().await?;

			return Ok(result);
		}

		let (decision, metadata) = self.resolve_update_decision(&mut tx, ctx, &note).await?;
		let base_decision =
			policy::base_decision_for_update(&decision, structured_present, graph_present);
		let (policy_decision, decision_policy_rule, min_confidence, min_importance) =
			policy::resolve_policy_for_update(&self.cfg, ctx.scope, &note, base_decision);
		let note_id = decision.note_id();
		let ignore_reason_code = policy::ignore_reason_code_for_policy(
			base_decision,
			policy_decision,
			metadata.matched_dup,
		);
		let (result, note_op, note_version_id) = policy::apply_policy_result(
			self,
			&mut tx,
			&decision,
			ctx,
			&note,
			note_id,
			policy_decision,
			ignore_reason_code,
		)
		.await?;
		let mut result = result;

		result.write_policy_audit = write_policy_audit.clone();

		audit::record_ingest_decision(
			&mut tx,
			&self.cfg,
			ctx,
			&note,
			result.note_id,
			note_version_id,
			base_decision,
			result.policy_decision,
			note_op,
			result.reason_code.as_deref(),
			decision_policy_rule.as_deref(),
			metadata.similarity_best,
			metadata.key_match,
			metadata.matched_dup,
			min_confidence,
			min_importance,
			write_policy_audit,
		)
		.await?;

		tx.commit().await?;

		Ok(result)
	}

	async fn resolve_update_decision(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		ctx: &AddNoteContext<'_>,
		note: &AddNoteInput,
	) -> Result<(UpdateDecision, UpdateDecisionMetadata)> {
		let decision = crate::resolve_update(
			&mut **tx,
			ResolveUpdateArgs {
				cfg: &self.cfg,
				providers: &self.providers,
				tenant_id: ctx.tenant_id,
				project_id: ctx.project_id,
				agent_id: ctx.agent_id,
				scope: ctx.scope,
				note_type: note.r#type.as_str(),
				key: note.key.as_deref(),
				text: note.text.as_str(),
				now: ctx.now,
			},
		)
		.await?;
		let metadata = decision.metadata();

		Ok((decision, metadata))
	}
}
