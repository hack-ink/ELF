use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
	ElfService, Error, InsertVersionArgs, NoteOp, ResolveUpdateArgs, Result, UpdateDecision,
	UpdateDecisionMetadata, access, structured_fields::StructuredFields,
};
use elf_config::Config;
use elf_domain::{english_gate, memory_policy::MemoryPolicyDecision, ttl};
use elf_storage::models::MemoryNote;

const REJECT_STRUCTURED_INVALID: &str = "REJECT_STRUCTURED_INVALID";
const IGNORE_DUPLICATE: &str = "IGNORE_DUPLICATE";
const IGNORE_POLICY_THRESHOLD: &str = "IGNORE_POLICY_THRESHOLD";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddNoteRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: String,
	pub notes: Vec<AddNoteInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddNoteInput {
	pub r#type: String,
	pub key: Option<String>,
	pub text: String,
	pub structured: Option<StructuredFields>,
	pub importance: f32,
	pub confidence: f32,
	pub ttl_days: Option<i64>,
	pub source_ref: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddNoteResult {
	pub note_id: Option<Uuid>,
	pub op: NoteOp,
	pub policy_decision: MemoryPolicyDecision,
	pub reason_code: Option<String>,
	pub field_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddNoteResponse {
	pub results: Vec<AddNoteResult>,
}

struct AddNoteContext<'a> {
	tenant_id: &'a str,
	project_id: &'a str,
	agent_id: &'a str,
	scope: &'a str,
	now: OffsetDateTime,
	embed_version: &'a str,
}

impl ElfService {
	pub async fn add_note(&self, req: AddNoteRequest) -> Result<AddNoteResponse> {
		validate_add_note_request(&req)?;

		let base_now = OffsetDateTime::now_utc();
		let embed_version = crate::embedding_version(&self.cfg);
		let AddNoteRequest { tenant_id, project_id, agent_id, scope, notes } = req;
		let effective_project_id =
			if scope.trim() == "org_shared" { access::ORG_PROJECT_ID } else { project_id.as_str() };
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
		let (structured_present, graph_present) =
			Self::structured_and_graph_present(note.structured.as_ref());
		let mut tx = self.db.pool.begin().await?;

		if let Some(result) = self.handle_rejection_paths(&mut tx, ctx, &note).await? {
			tx.commit().await?;

			return Ok(result);
		}

		let (decision, metadata) = self.resolve_update_decision(ctx, &note).await?;
		let base_decision =
			Self::base_decision_for_update(&decision, structured_present, graph_present);
		let (policy_decision, decision_policy_rule, min_confidence, min_importance) =
			self.decide_policy_decision(ctx.scope, &note, base_decision);
		let note_id = decision.note_id();
		let ignore_reason_code =
			Self::ignore_reason_code(policy_decision, base_decision, metadata.matched_dup);
		let (result, note_op) = self
			.apply_policy_result(
				&mut tx,
				&decision,
				ctx,
				&note,
				note_id,
				policy_decision,
				ignore_reason_code,
			)
			.await?;

		self.record_ingest_decision(
			&mut tx,
			ctx,
			&note,
			result.note_id,
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
		)
		.await?;
		tx.commit().await?;

		Ok(result)
	}

	fn structured_and_graph_present(structured: Option<&StructuredFields>) -> (bool, bool) {
		let structured_present = structured.is_some_and(|s| !s.is_effectively_empty());
		let graph_present = structured.is_some_and(StructuredFields::has_graph_fields);

		(structured_present, graph_present)
	}

	async fn handle_rejection_paths(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		ctx: &AddNoteContext<'_>,
		note: &AddNoteInput,
	) -> Result<Option<AddNoteResult>> {
		if let Some(result) = reject_note_if_structured_invalid(note) {
			self.record_ingest_decision(
				tx,
				ctx,
				note,
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
			)
			.await?;

			return Ok(Some(result));
		}
		if let Some(result) = reject_note_if_writegate_rejects(&self.cfg, ctx.scope, note) {
			self.record_ingest_decision(
				tx,
				ctx,
				note,
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
			)
			.await?;

			return Ok(Some(result));
		}

		Ok(None)
	}

	async fn resolve_update_decision(
		&self,
		ctx: &AddNoteContext<'_>,
		note: &AddNoteInput,
	) -> Result<(UpdateDecision, UpdateDecisionMetadata)> {
		let decision = crate::resolve_update(
			&self.db.pool,
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

	fn decide_policy_decision(
		&self,
		scope: &str,
		note: &AddNoteInput,
		base_decision: MemoryPolicyDecision,
	) -> (MemoryPolicyDecision, Option<String>, Option<f32>, Option<f32>) {
		if matches!(base_decision, MemoryPolicyDecision::Remember | MemoryPolicyDecision::Update) {
			let policy_eval = elf_domain::memory_policy::evaluate_memory_policy(
				&self.cfg,
				note.r#type.as_str(),
				scope,
				f64::from(note.confidence),
				f64::from(note.importance),
				base_decision,
			);
			let decision_policy_rule = policy_eval.matched_rule.and_then(|rule| {
				Self::policy_rule_id(rule.note_type.as_deref(), rule.scope.as_deref())
			});
			let min_confidence = policy_eval.matched_rule.and_then(|rule| rule.min_confidence);
			let min_importance = policy_eval.matched_rule.and_then(|rule| rule.min_importance);

			(policy_eval.decision, decision_policy_rule, min_confidence, min_importance)
		} else {
			(MemoryPolicyDecision::Ignore, None, None, None)
		}
	}

	fn ignore_reason_code(
		policy_decision: MemoryPolicyDecision,
		base_decision: MemoryPolicyDecision,
		matched_dup: bool,
	) -> Option<&'static str> {
		if !matches!(policy_decision, MemoryPolicyDecision::Ignore) {
			return None;
		}

		match base_decision {
			MemoryPolicyDecision::Remember | MemoryPolicyDecision::Update =>
				Some(IGNORE_POLICY_THRESHOLD),
			MemoryPolicyDecision::Ignore if matched_dup => Some(IGNORE_DUPLICATE),
			_ => None,
		}
	}

	#[allow(clippy::too_many_arguments)]
	async fn apply_policy_result(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		decision: &UpdateDecision,
		ctx: &AddNoteContext<'_>,
		note: &AddNoteInput,
		note_id: Uuid,
		policy_decision: MemoryPolicyDecision,
		ignore_reason_code: Option<&'static str>,
	) -> Result<(AddNoteResult, NoteOp)> {
		let should_apply = matches!(
			policy_decision,
			MemoryPolicyDecision::Remember | MemoryPolicyDecision::Update
		);

		if should_apply {
			let result = match decision {
				UpdateDecision::Add { .. } => {
					self.handle_add_note_add(tx, ctx, note, note_id).await?;

					AddNoteResult {
						note_id: Some(note_id),
						op: NoteOp::Add,
						policy_decision,
						reason_code: None,
						field_path: None,
					}
				},
				UpdateDecision::Update { .. } =>
					self.handle_add_note_update(
						tx,
						note,
						note_id,
						ctx.agent_id,
						ctx.now,
						policy_decision,
					)
					.await?,
				UpdateDecision::None { .. } => {
					let mut none_result = self
						.handle_add_note_none(
							tx,
							ctx,
							note,
							note_id,
							ctx.now,
							ctx.embed_version,
							policy_decision,
						)
						.await?;

					none_result.policy_decision = policy_decision;

					none_result
				},
			};
			let note_op = result.op;

			Ok((result, note_op))
		} else {
			let mut result = AddNoteResult {
				note_id: Some(note_id),
				op: NoteOp::None,
				policy_decision,
				reason_code: ignore_reason_code.map(str::to_string),
				field_path: None,
			};

			match decision {
				UpdateDecision::Add { .. } => {
					result.note_id = None;
				},
				UpdateDecision::Update { .. } | UpdateDecision::None { .. } => {},
			}

			Ok((result, NoteOp::None))
		}
	}

	#[allow(clippy::too_many_arguments)]
	async fn record_ingest_decision(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		ctx: &AddNoteContext<'_>,
		note: &AddNoteInput,
		note_id: Option<Uuid>,
		base_decision: MemoryPolicyDecision,
		policy_decision: MemoryPolicyDecision,
		note_op: NoteOp,
		reason_code: Option<&str>,
		policy_rule: Option<&str>,
		similarity_best: Option<f32>,
		key_match: bool,
		matched_dup: bool,
		min_confidence: Option<f32>,
		min_importance: Option<f32>,
	) -> Result<()> {
		let decision = crate::ingest_audit::IngestAuditArgs {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			scope: ctx.scope,
			pipeline: "add_note",
			note_type: note.r#type.as_str(),
			note_key: note.key.as_deref(),
			note_id,
			base_decision,
			policy_decision,
			note_op,
			reason_code,
			similarity_best,
			key_match,
			matched_dup,
			dup_sim_threshold: self.cfg.memory.dup_sim_threshold,
			update_sim_threshold: self.cfg.memory.update_sim_threshold,
			confidence: note.confidence,
			importance: note.importance,
			structured_present: note.structured.as_ref().is_some_and(|s| !s.is_effectively_empty()),
			graph_present: note.structured.as_ref().is_some_and(StructuredFields::has_graph_fields),
			policy_rule,
			min_confidence,
			min_importance,
			ts: ctx.now,
		};

		crate::ingest_audit::insert_ingest_decision(tx, decision).await
	}

	fn base_decision_for_update(
		decision: &UpdateDecision,
		structured_present: bool,
		graph_present: bool,
	) -> MemoryPolicyDecision {
		match decision {
			UpdateDecision::Update { .. } => MemoryPolicyDecision::Update,
			UpdateDecision::Add { .. } => MemoryPolicyDecision::Remember,
			UpdateDecision::None { .. } =>
				if structured_present || graph_present {
					MemoryPolicyDecision::Update
				} else {
					MemoryPolicyDecision::Ignore
				},
		}
	}

	fn policy_rule_id(note_type: Option<&str>, scope: Option<&str>) -> Option<String> {
		match (note_type, scope) {
			(Some(note_type), Some(scope)) => Some(format!("note_type={note_type},scope={scope}")),
			(Some(note_type), None) => Some(format!("note_type={note_type}")),
			(None, Some(scope)) => Some(format!("scope={scope}")),
			(None, None) => None,
		}
	}

	async fn handle_add_note_add(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		ctx: &AddNoteContext<'_>,
		note: &AddNoteInput,
		note_id: Uuid,
	) -> Result<()> {
		access::ensure_active_project_scope_grant(
			&mut **tx,
			ctx.tenant_id,
			ctx.project_id,
			ctx.scope,
			ctx.agent_id,
		)
		.await?;

		let expires_at =
			ttl::compute_expires_at(note.ttl_days, note.r#type.as_str(), &self.cfg, ctx.now);
		let memory_note = MemoryNote {
			note_id,
			tenant_id: ctx.tenant_id.to_string(),
			project_id: ctx.project_id.to_string(),
			agent_id: ctx.agent_id.to_string(),
			scope: ctx.scope.to_string(),
			r#type: note.r#type.clone(),
			key: note.key.clone(),
			text: note.text.clone(),
			importance: note.importance,
			confidence: note.confidence,
			status: "active".to_string(),
			created_at: ctx.now,
			updated_at: ctx.now,
			expires_at,
			embedding_version: ctx.embed_version.to_string(),
			source_ref: note.source_ref.clone(),
			hit_count: 0,
			last_hit_at: None,
		};

		insert_memory_note_tx(tx, &memory_note).await?;

		crate::insert_version(
			&mut **tx,
			InsertVersionArgs {
				note_id: memory_note.note_id,
				op: "ADD",
				prev_snapshot: None,
				new_snapshot: Some(crate::note_snapshot(&memory_note)),
				reason: "add_note",
				actor: ctx.agent_id,
				ts: ctx.now,
			},
		)
		.await?;

		self.upsert_structured_and_enqueue_outbox(
			tx,
			note,
			memory_note.note_id,
			ctx.embed_version,
			ctx.now,
		)
		.await?;
		self.persist_graph_fields_if_present(
			tx,
			ctx.tenant_id,
			ctx.project_id,
			ctx.agent_id,
			ctx.scope,
			memory_note.note_id,
			ctx.now,
			note.structured.as_ref(),
		)
		.await?;

		Ok(())
	}

	async fn handle_add_note_update(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		note: &AddNoteInput,
		note_id: Uuid,
		agent_id: &str,
		now: OffsetDateTime,
		policy_decision: MemoryPolicyDecision,
	) -> Result<AddNoteResult> {
		let mut existing: MemoryNote = sqlx::query_as::<_, MemoryNote>(
			"SELECT * FROM memory_notes WHERE note_id = $1 FOR UPDATE",
		)
		.bind(note_id)
		.fetch_one(&mut **tx)
		.await?;
		let prev_snapshot = crate::note_snapshot(&existing);
		let requested_ttl = note.ttl_days.filter(|days| *days > 0);
		let expires_at = match requested_ttl {
			Some(ttl) => ttl::compute_expires_at(Some(ttl), note.r#type.as_str(), &self.cfg, now),
			None => existing.expires_at,
		};
		let expires_match = requested_ttl.map_or(existing.expires_at == expires_at, |ttl_days| {
			match existing.expires_at {
				Some(existing_expires_at) => {
					let existing_ttl =
						(existing_expires_at - existing.updated_at).whole_days() as i64;

					existing_ttl == ttl_days
				},
				None => false,
			}
		});
		let float_eps = 1e-6_f32;
		let unchanged = existing.text == note.text
			&& (existing.importance - note.importance).abs() <= float_eps
			&& (existing.confidence - note.confidence).abs() <= float_eps
			&& expires_match
			&& existing.source_ref == note.source_ref;

		if unchanged {
			return Ok(AddNoteResult {
				note_id: Some(note_id),
				op: NoteOp::None,
				policy_decision: MemoryPolicyDecision::Ignore,
				reason_code: None,
				field_path: None,
			});
		}

		access::ensure_active_project_scope_grant(
			&mut **tx,
			existing.tenant_id.as_str(),
			existing.project_id.as_str(),
			existing.scope.as_str(),
			existing.agent_id.as_str(),
		)
		.await?;

		existing.text = note.text.clone();
		existing.importance = note.importance;
		existing.confidence = note.confidence;
		existing.updated_at = now;
		existing.expires_at = expires_at;
		existing.source_ref = note.source_ref.clone();

		update_memory_note_tx(tx, &existing).await?;

		crate::insert_version(
			&mut **tx,
			InsertVersionArgs {
				note_id: existing.note_id,
				op: "UPDATE",
				prev_snapshot: Some(prev_snapshot),
				new_snapshot: Some(crate::note_snapshot(&existing)),
				reason: "add_note",
				actor: agent_id,
				ts: now,
			},
		)
		.await?;

		self.persist_graph_fields_if_present(
			tx,
			existing.tenant_id.as_str(),
			existing.project_id.as_str(),
			existing.agent_id.as_str(),
			existing.scope.as_str(),
			existing.note_id,
			now,
			note.structured.as_ref(),
		)
		.await?;
		self.upsert_structured_and_enqueue_outbox(
			tx,
			note,
			existing.note_id,
			existing.embedding_version.as_str(),
			now,
		)
		.await?;

		Ok(AddNoteResult {
			note_id: Some(note_id),
			op: NoteOp::Update,
			policy_decision,
			reason_code: None,
			field_path: None,
		})
	}

	#[allow(clippy::too_many_arguments)]
	async fn handle_add_note_none(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		ctx: &AddNoteContext<'_>,
		note: &AddNoteInput,
		note_id: Uuid,
		now: OffsetDateTime,
		embed_version: &str,
		policy_decision: MemoryPolicyDecision,
	) -> Result<AddNoteResult> {
		let mut should_update = false;

		if let Some(structured) = note.structured.as_ref() {
			if !structured.is_effectively_empty() {
				crate::structured_fields::upsert_structured_fields_tx(tx, note_id, structured, now)
					.await?;
				crate::enqueue_outbox_tx(&mut **tx, note_id, "UPSERT", embed_version, now).await?;

				should_update = true;
			}
			if structured.has_graph_fields() {
				self.persist_graph_fields_if_present(
					tx,
					ctx.tenant_id,
					ctx.project_id,
					ctx.agent_id,
					ctx.scope,
					note_id,
					now,
					Some(structured),
				)
				.await?;

				should_update = true;
			}
		}

		if should_update {
			if matches!(ctx.scope, "project_shared" | "org_shared") {
				access::ensure_active_project_scope_grant(
					&mut **tx,
					ctx.tenant_id,
					ctx.project_id,
					ctx.scope,
					ctx.agent_id,
				)
				.await?;
			}

			return Ok(AddNoteResult {
				note_id: Some(note_id),
				op: NoteOp::Update,
				policy_decision,
				reason_code: None,
				field_path: None,
			});
		}

		Ok(AddNoteResult {
			note_id: Some(note_id),
			op: NoteOp::None,
			policy_decision,
			reason_code: None,
			field_path: None,
		})
	}

	#[allow(clippy::too_many_arguments)]
	async fn persist_graph_fields_if_present(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		tenant_id: &str,
		project_id: &str,
		agent_id: &str,
		scope: &str,
		note_id: Uuid,
		now: OffsetDateTime,
		structured: Option<&StructuredFields>,
	) -> Result<()> {
		let Some(structured) = structured else {
			return Ok(());
		};

		if !structured.has_graph_fields() {
			return Ok(());
		}

		crate::graph_ingestion::persist_graph_fields_tx(
			tx, tenant_id, project_id, agent_id, scope, note_id, structured, now,
		)
		.await?;

		Ok(())
	}

	async fn upsert_structured_and_enqueue_outbox(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		note: &AddNoteInput,
		note_id: Uuid,
		embed_version: &str,
		now: OffsetDateTime,
	) -> Result<()> {
		if let Some(structured) = note.structured.as_ref()
			&& !structured.is_effectively_empty()
		{
			crate::structured_fields::upsert_structured_fields_tx(tx, note_id, structured, now)
				.await?;
		}

		crate::enqueue_outbox_tx(&mut **tx, note_id, "UPSERT", embed_version, now).await?;

		Ok(())
	}
}

fn validate_add_note_request(req: &AddNoteRequest) -> Result<()> {
	if req.notes.is_empty() {
		return Err(Error::InvalidRequest { message: "Notes list is empty.".to_string() });
	}
	if req.tenant_id.trim().is_empty()
		|| req.project_id.trim().is_empty()
		|| req.agent_id.trim().is_empty()
		|| req.scope.trim().is_empty()
	{
		return Err(Error::InvalidRequest {
			message: "tenant_id, project_id, agent_id, and scope are required.".to_string(),
		});
	}

	for (idx, note) in req.notes.iter().enumerate() {
		if !english_gate::is_english_natural_language(note.text.as_str()) {
			return Err(Error::NonEnglishInput { field: format!("$.notes[{idx}].text") });
		}

		if let Some(key) = note.key.as_ref()
			&& !english_gate::is_english_identifier(key)
		{
			return Err(Error::NonEnglishInput { field: format!("$.notes[{idx}].key") });
		}
		if let Some(path) = find_non_english_path_in_structured(
			note.structured.as_ref(),
			&format!("$.notes[{idx}].structured"),
		) {
			return Err(Error::NonEnglishInput { field: path });
		}
		if let Some(path) =
			find_non_english_path(&note.source_ref, &format!("$.notes[{idx}].source_ref"))
		{
			return Err(Error::NonEnglishInput { field: path });
		}
	}

	Ok(())
}

fn reject_note_if_structured_invalid(note: &AddNoteInput) -> Option<AddNoteResult> {
	if let Some(structured) = note.structured.as_ref()
		&& let Err(err) = crate::structured_fields::validate_structured_fields(
			structured,
			note.text.as_str(),
			&note.source_ref,
			None,
		) {
		tracing::info!(error = %err, "Rejecting note due to invalid structured fields.");

		let field_path = extract_structured_rejection_field_path(&err);

		return Some(AddNoteResult {
			note_id: None,
			op: NoteOp::Rejected,
			policy_decision: MemoryPolicyDecision::Reject,
			reason_code: Some(REJECT_STRUCTURED_INVALID.to_string()),
			field_path,
		});
	}

	None
}

fn reject_note_if_writegate_rejects(
	cfg: &Config,
	scope: &str,
	note: &AddNoteInput,
) -> Option<AddNoteResult> {
	let gate_input = elf_domain::writegate::NoteInput {
		note_type: note.r#type.clone(),
		scope: scope.to_string(),
		text: note.text.clone(),
	};

	if let Err(code) = elf_domain::writegate::writegate(&gate_input, cfg) {
		return Some(AddNoteResult {
			note_id: None,
			op: NoteOp::Rejected,
			policy_decision: MemoryPolicyDecision::Reject,
			reason_code: Some(crate::writegate_reason_code(code).to_string()),
			field_path: None,
		});
	}

	None
}

fn find_non_english_path_in_structured(
	structured: Option<&StructuredFields>,
	base: &str,
) -> Option<String> {
	let structured = structured?;

	if let Some(summary) = structured.summary.as_ref()
		&& !english_gate::is_english_natural_language(summary)
	{
		return Some(format!("{base}.summary"));
	}
	if let Some(items) = structured.facts.as_ref() {
		for (idx, item) in items.iter().enumerate() {
			if !english_gate::is_english_natural_language(item) {
				return Some(format!("{base}.facts[{idx}]"));
			}
		}
	}
	if let Some(items) = structured.concepts.as_ref() {
		for (idx, item) in items.iter().enumerate() {
			if !english_gate::is_english_natural_language(item) {
				return Some(format!("{base}.concepts[{idx}]"));
			}
		}
	}
	if let Some(items) = structured.entities.as_ref() {
		for (idx, entity) in items.iter().enumerate() {
			let base = format!("{base}.entities[{idx}]");

			if let Some(canonical) = entity.canonical.as_ref()
				&& !english_gate::is_english_natural_language(canonical)
			{
				return Some(format!("{base}.canonical"));
			}
			if let Some(kind) = entity.kind.as_ref()
				&& !english_gate::is_english_natural_language(kind)
			{
				return Some(format!("{base}.kind"));
			}
			if let Some(aliases) = entity.aliases.as_ref() {
				for (alias_idx, alias) in aliases.iter().enumerate() {
					if !english_gate::is_english_natural_language(alias) {
						return Some(format!("{base}.aliases[{alias_idx}]"));
					}
				}
			}
		}
	}
	if let Some(items) = structured.relations.as_ref() {
		for (idx, relation) in items.iter().enumerate() {
			let base = format!("{base}.relations[{idx}]");

			if let Some(subject) = relation.subject.as_ref() {
				let subject_base = format!("{base}.subject");

				if let Some(canonical) = subject.canonical.as_ref()
					&& !english_gate::is_english_natural_language(canonical)
				{
					return Some(format!("{subject_base}.canonical"));
				}
				if let Some(kind) = subject.kind.as_ref()
					&& !english_gate::is_english_natural_language(kind)
				{
					return Some(format!("{subject_base}.kind"));
				}
				if let Some(aliases) = subject.aliases.as_ref() {
					for (alias_idx, alias) in aliases.iter().enumerate() {
						if !english_gate::is_english_natural_language(alias) {
							return Some(format!("{subject_base}.aliases[{alias_idx}]"));
						}
					}
				}
			}
			if let Some(predicate) = relation.predicate.as_ref()
				&& !english_gate::is_english_natural_language(predicate)
			{
				return Some(format!("{base}.predicate"));
			}
			if let Some(object) = relation.object.as_ref() {
				if let Some(entity) = object.entity.as_ref() {
					let object_base = format!("{base}.object.entity");

					if let Some(canonical) = entity.canonical.as_ref()
						&& !english_gate::is_english_natural_language(canonical)
					{
						return Some(format!("{object_base}.canonical"));
					}
					if let Some(kind) = entity.kind.as_ref()
						&& !english_gate::is_english_natural_language(kind)
					{
						return Some(format!("{object_base}.kind"));
					}
					if let Some(aliases) = entity.aliases.as_ref() {
						for (alias_idx, alias) in aliases.iter().enumerate() {
							if !english_gate::is_english_natural_language(alias) {
								return Some(format!("{object_base}.aliases[{alias_idx}]"));
							}
						}
					}
				}
				if let Some(value) = object.value.as_ref()
					&& !english_gate::is_english_natural_language(value)
				{
					return Some(format!("{base}.object.value"));
				}
			}
		}
	}

	None
}

fn find_non_english_path(value: &Value, path: &str) -> Option<String> {
	find_non_english_path_inner(value, path, false)
}

fn find_non_english_path_inner(
	value: &Value,
	path: &str,
	is_identifier_lane: bool,
) -> Option<String> {
	fn has_english_gate(text: &str, is_identifier_lane: bool) -> bool {
		if is_identifier_lane && !text.contains(char::is_whitespace) {
			return english_gate::is_english_identifier(text);
		}

		english_gate::is_english_natural_language(text)
	}

	match value {
		Value::String(text) =>
			if !has_english_gate(text, is_identifier_lane) {
				Some(path.to_string())
			} else {
				None
			},
		Value::Array(items) => {
			for (idx, item) in items.iter().enumerate() {
				let child_path = format!("{path}[{idx}]");

				if let Some(found) =
					find_non_english_path_inner(item, &child_path, is_identifier_lane)
				{
					return Some(found);
				}
			}

			None
		},
		Value::Object(map) => {
			for (key, value) in map.iter() {
				let identifier_lane = is_identifier_lane
					|| matches!(key.as_str(), "ref" | "schema" | "resolver" | "hashes" | "state");
				let child_path = format!("{path}[\"{}\"]", escape_json_path_key(key));

				if let Some(found) =
					find_non_english_path_inner(value, &child_path, identifier_lane)
				{
					return Some(found);
				}
			}

			None
		},
		_ => None,
	}
}

fn escape_json_path_key(key: &str) -> String {
	key.replace('\\', "\\\\").replace('"', "\\\"")
}

fn extract_structured_rejection_field_path(err: &Error) -> Option<String> {
	match err {
		Error::NonEnglishInput { field } => Some(field.clone()),
		Error::InvalidRequest { message } if message.starts_with("structured.") =>
			message.split_whitespace().next().map(ToString::to_string),
		_ => None,
	}
}

async fn insert_memory_note_tx(
	tx: &mut Transaction<'_, Postgres>,
	memory_note: &MemoryNote,
) -> Result<()> {
	sqlx::query(
		"\
INSERT INTO memory_notes (
	note_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	type,
	key,
	text,
	importance,
	confidence,
	status,
	created_at,
	updated_at,
	expires_at,
	embedding_version,
	source_ref,
	hit_count,
	last_hit_at
)
VALUES (
	$1,
	$2,
	$3,
	$4,
	$5,
	$6,
	$7,
	$8,
	$9,
	$10,
	$11,
	$12,
	$13,
	$14,
	$15,
	$16,
	$17,
	$18
)",
	)
	.bind(memory_note.note_id)
	.bind(memory_note.tenant_id.as_str())
	.bind(memory_note.project_id.as_str())
	.bind(memory_note.agent_id.as_str())
	.bind(memory_note.scope.as_str())
	.bind(memory_note.r#type.as_str())
	.bind(memory_note.key.as_deref())
	.bind(memory_note.text.as_str())
	.bind(memory_note.importance)
	.bind(memory_note.confidence)
	.bind(memory_note.status.as_str())
	.bind(memory_note.created_at)
	.bind(memory_note.updated_at)
	.bind(memory_note.expires_at)
	.bind(memory_note.embedding_version.as_str())
	.bind(&memory_note.source_ref)
	.bind(memory_note.hit_count)
	.bind(memory_note.last_hit_at)
	.execute(&mut **tx)
	.await?;

	Ok(())
}

async fn update_memory_note_tx(
	tx: &mut Transaction<'_, Postgres>,
	memory_note: &MemoryNote,
) -> Result<()> {
	sqlx::query(
		"\
UPDATE memory_notes
SET
	text = $1,
	importance = $2,
	confidence = $3,
	updated_at = $4,
	expires_at = $5,
	source_ref = $6
WHERE note_id = $7",
	)
	.bind(memory_note.text.as_str())
	.bind(memory_note.importance)
	.bind(memory_note.confidence)
	.bind(memory_note.updated_at)
	.bind(memory_note.expires_at)
	.bind(&memory_note.source_ref)
	.bind(memory_note.note_id)
	.execute(&mut **tx)
	.await?;

	Ok(())
}

#[cfg(test)]
mod english_gate_tests {
	use crate::{
		Error,
		add_note::{AddNoteInput, AddNoteRequest, validate_add_note_request},
	};

	#[test]
	fn accepts_identifier_like_source_ref_ref_field() {
		validate_add_note_request(&AddNoteRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "agent_private".to_string(),
			notes: vec![AddNoteInput {
				r#type: "fact".to_string(),
				key: Some("test_key".to_string()),
				text: "English text".to_string(),
				structured: None,
				importance: 0.5,
				confidence: 0.9,
				ttl_days: None,
				source_ref: serde_json::json!({"ref": "packages/elf-service/src/docs.rs:661"}),
			}],
		})
		.expect("Expected identifier-like source_ref to be accepted.");
	}

	#[test]
	fn rejects_non_english_source_ref_hints_quote() {
		let req = AddNoteRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "agent_private".to_string(),
			notes: vec![AddNoteInput {
				r#type: "fact".to_string(),
				key: Some("test_key".to_string()),
				text: "English text".to_string(),
				structured: None,
				importance: 0.5,
				confidence: 0.9,
				ttl_days: None,
				source_ref: serde_json::json!({"hints": {"quote": "\u{4f60}\u{597d}\u{4e16}\u{754c}"}}),
			}],
		};
		let err = validate_add_note_request(&req).expect_err(
			"Expected non-English free-text under source_ref.hints.quote to be rejected.",
		);

		match err {
			Error::NonEnglishInput { field } => {
				assert_eq!(field, "$.notes[0].source_ref[\"hints\"][\"quote\"]")
			},
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn rejects_long_non_english_note_text() {
		let req = AddNoteRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "agent_private".to_string(),
			notes: vec![AddNoteInput {
				r#type: "fact".to_string(),
				key: Some("test_key".to_string()),
				text: "Bonjour, je veux m'assurer que ce texte est suffisamment long et riche en lettres pour declencher la detection de langue. Merci beaucoup."
					.to_string(),
				structured: None,
				importance: 0.5,
				confidence: 0.9,
				ttl_days: None,
				source_ref: serde_json::json!({}),
			}],
		};
		let err = validate_add_note_request(&req).expect_err("Expected English gate rejection.");

		assert!(matches!(
			err,
			Error::NonEnglishInput { field } if field == "$.notes[0].text"
		));
	}
}
