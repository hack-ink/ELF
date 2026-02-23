use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgConnection, Postgres, Transaction};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
	ElfService, Error, InsertVersionArgs, NoteOp, REJECT_EVIDENCE_MISMATCH, ResolveUpdateArgs,
	Result, UpdateDecision, access, structured_fields::StructuredFields,
};
use elf_config::Config;
use elf_domain::{
	cjk, evidence,
	memory_policy::{self, MemoryPolicyDecision},
	ttl,
};
use elf_storage::models::MemoryNote;

const REJECT_STRUCTURED_INVALID: &str = "REJECT_STRUCTURED_INVALID";
const IGNORE_DUPLICATE: &str = "IGNORE_DUPLICATE";
const IGNORE_POLICY_THRESHOLD: &str = "IGNORE_POLICY_THRESHOLD";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventMessage {
	pub role: String,
	pub content: String,
	pub ts: Option<String>,
	pub msg_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddEventRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: Option<String>,
	pub dry_run: Option<bool>,
	pub messages: Vec<EventMessage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddEventResult {
	pub note_id: Option<Uuid>,
	pub op: NoteOp,
	pub policy_decision: MemoryPolicyDecision,
	pub reason_code: Option<String>,
	pub reason: Option<String>,
	pub field_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddEventResponse {
	pub extracted: Value,
	pub results: Vec<AddEventResult>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ExtractorOutput {
	pub notes: Vec<ExtractedNote>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ExtractedNote {
	pub r#type: Option<String>,
	pub key: Option<String>,
	pub text: Option<String>,
	pub structured: Option<StructuredFields>,
	pub importance: Option<f32>,
	pub confidence: Option<f32>,
	pub ttl_days: Option<i64>,
	pub scope_suggestion: Option<String>,
	pub evidence: Option<Vec<EvidenceQuote>>,
	pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EvidenceQuote {
	pub message_index: usize,
	pub quote: String,
}

struct NoteProcessingData {
	note_type: String,
	text: String,
	structured: Option<StructuredFields>,
	importance: f32,
	confidence: f32,
	reason: Option<String>,
	ttl_days: Option<i64>,
	scope: String,
	evidence: Vec<EvidenceQuote>,
	structured_present: bool,
	graph_present: bool,
}
impl NoteProcessingData {
	fn from_request_and_note(req: &AddEventRequest, note: &ExtractedNote) -> Self {
		let note_type = note.r#type.clone().unwrap_or_default();
		let text = note.text.clone().unwrap_or_default();
		let structured = note.structured.clone();
		let structured_present =
			structured.as_ref().is_some_and(|value| !value.is_effectively_empty());
		let graph_present = structured.as_ref().is_some_and(StructuredFields::has_graph_fields);

		Self {
			note_type,
			text,
			structured,
			importance: note.importance.unwrap_or(0.0),
			confidence: note.confidence.unwrap_or(0.0),
			reason: note.reason.clone(),
			ttl_days: note.ttl_days,
			scope: req.scope.clone().or(note.scope_suggestion.clone()).unwrap_or_default(),
			evidence: note.evidence.clone().unwrap_or_default(),
			structured_present,
			graph_present,
		}
	}
}

struct PersistExtractedNoteArgs<'a> {
	req: &'a AddEventRequest,
	project_id: &'a str,
	structured: Option<&'a StructuredFields>,
	key: Option<&'a str>,
	reason: Option<&'a String>,
	note_type: &'a str,
	text: &'a str,
	scope: &'a str,
	importance: f32,
	confidence: f32,
	expires_at: Option<OffsetDateTime>,
	source_ref: Value,
	now: OffsetDateTime,
	embed_version: &'a str,
}

struct AddEventContext<'a> {
	tenant_id: &'a str,
	project_id: &'a str,
	agent_id: &'a str,
	scope: &'a str,
	now: OffsetDateTime,
}

impl ElfService {
	pub async fn add_event(&self, req: AddEventRequest) -> Result<AddEventResponse> {
		validate_add_event_request(&req)?;

		let messages_json = build_extractor_messages(
			&req.messages,
			self.cfg.memory.max_notes_per_add_event,
			self.cfg.memory.max_note_chars,
		)?;
		let extracted_raw = self
			.providers
			.extractor
			.extract(&self.cfg.providers.llm_extractor, &messages_json)
			.await?;
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
		let message_texts: Vec<String> = req.messages.iter().map(|m| m.content.clone()).collect();
		let mut results = Vec::with_capacity(extracted.notes.len());

		for (note_idx, note) in extracted.notes.into_iter().enumerate() {
			let now = base_now + Duration::microseconds(note_idx as i64);

			results.push(
				self.process_extracted_note(
					&req,
					&message_texts,
					note,
					now,
					embed_version.as_str(),
					dry_run,
				)
				.await?,
			);
		}

		Ok(AddEventResponse { extracted: extracted_json, results })
	}

	async fn process_extracted_note(
		&self,
		req: &AddEventRequest,
		message_texts: &[String],
		note: ExtractedNote,
		now: OffsetDateTime,
		embed_version: &str,
		dry_run: bool,
	) -> Result<AddEventResult> {
		let note_data = NoteProcessingData::from_request_and_note(req, &note);
		let effective_project_id = if note_data.scope.trim() == "org_shared" {
			access::ORG_PROJECT_ID
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

		if let Some(result) = self
			.record_extracted_note_rejections(&mut tx, &ctx, &note, &note_data, message_texts)
			.await?
		{
			tx.commit().await?;

			return Ok(result);
		}

		let decision =
			self.resolve_extracted_note_update(&note, req, &note_data, &mut tx, now).await?;
		let metadata = decision.metadata();
		let base_decision = base_decision_for_update(
			&decision,
			note_data.structured_present,
			note_data.graph_present,
		);
		let (policy_decision, decision_policy_rule, min_confidence, min_importance) =
			resolve_policy_for_update(&self.cfg, &note_data, base_decision);
		let ignore_reason_code =
			ignore_reason_code_for_policy(base_decision, policy_decision, metadata.matched_dup);
		let should_apply = matches!(
			policy_decision,
			MemoryPolicyDecision::Remember | MemoryPolicyDecision::Update
		);
		let mut result = build_result_from_decision(
			&decision,
			policy_decision,
			note_data.reason.clone(),
			note_data.structured_present || note_data.graph_present,
		);

		apply_policy_ignore_adjustments(
			&mut result,
			&decision,
			policy_decision,
			ignore_reason_code,
		);

		if should_apply && !dry_run {
			let persist_args = PersistExtractedNoteArgs {
				req,
				project_id: effective_project_id,
				structured: note_data.structured.as_ref(),
				key: note.key.as_deref(),
				reason: note.reason.as_ref(),
				note_type: note_data.note_type.as_str(),
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
				}),
				now,
				embed_version,
			};

			result = self
				.persist_extracted_note_decision(&mut tx, persist_args, decision, policy_decision)
				.await?;
		}

		record_ingest_decision(
			&mut tx,
			&self.cfg,
			&ctx,
			&note,
			note_data.note_type.as_str(),
			result.note_id,
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
			note_data.structured_present,
			note_data.graph_present,
		)
		.await?;

		tx.commit().await?;

		Ok(result)
	}

	async fn record_extracted_note_rejections(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		ctx: &AddEventContext<'_>,
		note: &ExtractedNote,
		note_data: &NoteProcessingData,
		message_texts: &[String],
	) -> Result<Option<AddEventResult>> {
		if let Some(result) = reject_extracted_note_if_evidence_invalid(
			&self.cfg,
			note.reason.as_ref(),
			&note_data.evidence,
			message_texts,
		) {
			record_ingest_decision(
				tx,
				&self.cfg,
				ctx,
				note,
				note_data.note_type.as_str(),
				None,
				MemoryPolicyDecision::Reject,
				MemoryPolicyDecision::Reject,
				NoteOp::Rejected,
				Some(REJECT_EVIDENCE_MISMATCH),
				None,
				None,
				false,
				false,
				None,
				None,
				note_data.structured_present,
				note_data.graph_present,
			)
			.await?;

			return Ok(Some(result));
		} else if let Some(result) = reject_extracted_note_if_structured_invalid(
			note_data.structured.as_ref(),
			note_data.text.as_str(),
			&note_data.evidence,
			note.reason.as_ref(),
		) {
			record_ingest_decision(
				tx,
				&self.cfg,
				ctx,
				note,
				note_data.note_type.as_str(),
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
				note_data.structured_present,
				note_data.graph_present,
			)
			.await?;

			return Ok(Some(result));
		} else if let Some(result) = reject_extracted_note_if_writegate_rejects(
			&self.cfg,
			note.reason.as_ref(),
			note_data.note_type.as_str(),
			note_data.scope.as_str(),
			note_data.text.as_str(),
		) {
			record_ingest_decision(
				tx,
				&self.cfg,
				ctx,
				note,
				note_data.note_type.as_str(),
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
				note_data.structured_present,
				note_data.graph_present,
			)
			.await?;

			return Ok(Some(result));
		}

		Ok(None)
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
					access::ORG_PROJECT_ID
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

	async fn persist_extracted_note_decision(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		args: PersistExtractedNoteArgs<'_>,
		decision: UpdateDecision,
		policy_decision: MemoryPolicyDecision,
	) -> Result<AddEventResult> {
		match (decision, args) {
			(UpdateDecision::Add { note_id, .. }, args) =>
				self.persist_extracted_note_add(tx, args, note_id, policy_decision).await,
			(UpdateDecision::Update { note_id, .. }, args) =>
				self.persist_extracted_note_update(tx, args, note_id, policy_decision).await,
			(UpdateDecision::None { note_id, .. }, args) =>
				self.persist_extracted_note_none(tx, args, note_id, policy_decision).await,
		}
	}

	async fn persist_extracted_note_add(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		args: PersistExtractedNoteArgs<'_>,
		note_id: Uuid,
		policy_decision: MemoryPolicyDecision,
	) -> Result<AddEventResult> {
		access::ensure_active_project_scope_grant(
			&mut **tx,
			args.req.tenant_id.as_str(),
			args.project_id,
			args.scope,
			args.req.agent_id.as_str(),
		)
		.await?;

		let memory_note = MemoryNote {
			note_id,
			tenant_id: args.req.tenant_id.clone(),
			project_id: args.project_id.to_string(),
			agent_id: args.req.agent_id.clone(),
			scope: args.scope.to_string(),
			r#type: args.note_type.to_string(),
			key: args.key.map(ToString::to_string),
			text: args.text.to_string(),
			importance: args.importance,
			confidence: args.confidence,
			status: "active".to_string(),
			created_at: args.now,
			updated_at: args.now,
			expires_at: args.expires_at,
			embedding_version: args.embed_version.to_string(),
			source_ref: args.source_ref,
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
				reason: "add_event",
				actor: args.req.agent_id.as_str(),
				ts: args.now,
			},
		)
		.await?;
		crate::enqueue_outbox_tx(
			&mut **tx,
			memory_note.note_id,
			"UPSERT",
			args.embed_version,
			args.now,
		)
		.await?;

		upsert_structured_fields_tx(tx, args.structured, memory_note.note_id, args.now).await?;

		if let Some(structured) = args.structured
			&& structured.has_graph_fields()
		{
			crate::graph_ingestion::persist_graph_fields_tx(
				tx,
				args.req.tenant_id.as_str(),
				args.project_id,
				args.req.agent_id.as_str(),
				args.scope,
				memory_note.note_id,
				structured,
				args.now,
			)
			.await?;
		}

		Ok(AddEventResult {
			note_id: Some(note_id),
			op: NoteOp::Add,
			policy_decision,
			reason_code: None,
			reason: args.reason.cloned(),
			field_path: None,
		})
	}

	async fn persist_extracted_note_update(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		args: PersistExtractedNoteArgs<'_>,
		note_id: Uuid,
		policy_decision: MemoryPolicyDecision,
	) -> Result<AddEventResult> {
		let mut existing: MemoryNote = sqlx::query_as::<_, MemoryNote>(
			"SELECT * FROM memory_notes WHERE note_id = $1 FOR UPDATE",
		)
		.bind(note_id)
		.fetch_one(&mut **tx)
		.await?;

		access::ensure_active_project_scope_grant(
			&mut **tx,
			existing.tenant_id.as_str(),
			existing.project_id.as_str(),
			existing.scope.as_str(),
			existing.agent_id.as_str(),
		)
		.await?;

		let prev_snapshot = crate::note_snapshot(&existing);

		existing.text = args.text.to_string();
		existing.importance = args.importance;
		existing.confidence = args.confidence;
		existing.updated_at = args.now;
		existing.expires_at = args.expires_at;
		existing.source_ref = args.source_ref;

		update_memory_note_tx(tx, &existing).await?;

		crate::insert_version(
			&mut **tx,
			InsertVersionArgs {
				note_id: existing.note_id,
				op: "UPDATE",
				prev_snapshot: Some(prev_snapshot),
				new_snapshot: Some(crate::note_snapshot(&existing)),
				reason: "add_event",
				actor: args.req.agent_id.as_str(),
				ts: args.now,
			},
		)
		.await?;
		crate::enqueue_outbox_tx(
			&mut **tx,
			existing.note_id,
			"UPSERT",
			existing.embedding_version.as_str(),
			args.now,
		)
		.await?;

		upsert_structured_fields_tx(tx, args.structured, existing.note_id, args.now).await?;

		if let Some(structured) = args.structured
			&& structured.has_graph_fields()
		{
			crate::graph_ingestion::persist_graph_fields_tx(
				tx,
				args.req.tenant_id.as_str(),
				existing.project_id.as_str(),
				args.req.agent_id.as_str(),
				args.scope,
				existing.note_id,
				structured,
				args.now,
			)
			.await?;
		}

		Ok(AddEventResult {
			note_id: Some(note_id),
			op: NoteOp::Update,
			policy_decision,
			reason_code: None,
			reason: args.reason.cloned(),
			field_path: None,
		})
	}

	async fn persist_extracted_note_none(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		args: PersistExtractedNoteArgs<'_>,
		note_id: Uuid,
		policy_decision: MemoryPolicyDecision,
	) -> Result<AddEventResult> {
		let mut did_update = false;

		if let Some(structured) = args.structured
			&& !structured.is_effectively_empty()
		{
			crate::structured_fields::upsert_structured_fields_tx(
				tx, note_id, structured, args.now,
			)
			.await?;
			crate::enqueue_outbox_tx(&mut **tx, note_id, "UPSERT", args.embed_version, args.now)
				.await?;

			did_update = true;
		}
		if let Some(structured) = args.structured
			&& structured.has_graph_fields()
		{
			crate::graph_ingestion::persist_graph_fields_tx(
				tx,
				args.req.tenant_id.as_str(),
				args.project_id,
				args.req.agent_id.as_str(),
				args.scope,
				note_id,
				structured,
				args.now,
			)
			.await?;

			did_update = true;
		}

		if did_update {
			if matches!(args.scope, "project_shared" | "org_shared") {
				access::ensure_active_project_scope_grant(
					&mut **tx,
					args.req.tenant_id.as_str(),
					args.project_id,
					args.scope,
					args.req.agent_id.as_str(),
				)
				.await?;
			}

			return Ok(AddEventResult {
				note_id: Some(note_id),
				op: NoteOp::Update,
				policy_decision,
				reason_code: None,
				reason: args.reason.cloned(),
				field_path: None,
			});
		}

		Ok(AddEventResult {
			note_id: Some(note_id),
			op: NoteOp::None,
			policy_decision,
			reason_code: None,
			reason: args.reason.cloned(),
			field_path: None,
		})
	}
}

fn resolve_policy_for_update(
	cfg: &Config,
	note_data: &NoteProcessingData,
	base_decision: MemoryPolicyDecision,
) -> (MemoryPolicyDecision, Option<String>, Option<f32>, Option<f32>) {
	if matches!(base_decision, MemoryPolicyDecision::Remember | MemoryPolicyDecision::Update) {
		let policy_eval = memory_policy::evaluate_memory_policy(
			cfg,
			note_data.note_type.as_str(),
			note_data.scope.as_str(),
			note_data.confidence as f64,
			note_data.importance as f64,
			base_decision,
		);
		let decision_policy_rule = policy_eval
			.matched_rule
			.and_then(|rule| policy_rule_id(rule.note_type.as_deref(), rule.scope.as_deref()));
		let min_confidence = policy_eval.matched_rule.and_then(|rule| rule.min_confidence);
		let min_importance = policy_eval.matched_rule.and_then(|rule| rule.min_importance);

		(policy_eval.decision, decision_policy_rule, min_confidence, min_importance)
	} else {
		(MemoryPolicyDecision::Ignore, None, None, None)
	}
}

fn ignore_reason_code_for_policy(
	base_decision: MemoryPolicyDecision,
	policy_decision: MemoryPolicyDecision,
	matched_duplicate: bool,
) -> Option<&'static str> {
	if !matches!(policy_decision, MemoryPolicyDecision::Ignore) {
		return None;
	}

	match base_decision {
		MemoryPolicyDecision::Remember | MemoryPolicyDecision::Update =>
			Some(IGNORE_POLICY_THRESHOLD),
		MemoryPolicyDecision::Ignore if matched_duplicate => Some(IGNORE_DUPLICATE),
		_ => None,
	}
}

fn build_result_from_decision(
	decision: &UpdateDecision,
	policy_decision: MemoryPolicyDecision,
	reason: Option<String>,
	structured_present: bool,
) -> AddEventResult {
	match decision {
		UpdateDecision::Add { note_id, .. } => AddEventResult {
			note_id: Some(*note_id),
			op: NoteOp::Add,
			policy_decision,
			reason_code: None,
			reason,
			field_path: None,
		},
		UpdateDecision::Update { note_id, .. } => AddEventResult {
			note_id: Some(*note_id),
			op: NoteOp::Update,
			policy_decision,
			reason_code: None,
			reason,
			field_path: None,
		},
		UpdateDecision::None { note_id, .. } => AddEventResult {
			note_id: Some(*note_id),
			op: if structured_present { NoteOp::Update } else { NoteOp::None },
			policy_decision,
			reason_code: None,
			reason,
			field_path: None,
		},
	}
}

fn apply_policy_ignore_adjustments(
	result: &mut AddEventResult,
	decision: &UpdateDecision,
	policy_decision: MemoryPolicyDecision,
	ignore_reason_code: Option<&str>,
) {
	if !matches!(policy_decision, MemoryPolicyDecision::Ignore) {
		return;
	}

	if let UpdateDecision::Add { .. } = decision {
		result.note_id = None;
	}

	result.op = NoteOp::None;
	result.reason_code = ignore_reason_code.map(str::to_string);
}

fn validate_add_event_request(req: &AddEventRequest) -> Result<()> {
	if req.messages.is_empty() {
		return Err(Error::InvalidRequest { message: "Messages list is empty.".to_string() });
	}
	if req.tenant_id.trim().is_empty()
		|| req.project_id.trim().is_empty()
		|| req.agent_id.trim().is_empty()
	{
		return Err(Error::InvalidRequest {
			message: "tenant_id, project_id, and agent_id are required.".to_string(),
		});
	}

	if let Some(scope) = req.scope.as_ref()
		&& scope.trim().is_empty()
	{
		return Err(Error::InvalidRequest {
			message: "scope must not be empty when provided.".to_string(),
		});
	}

	for (idx, msg) in req.messages.iter().enumerate() {
		if cjk::contains_cjk(msg.content.as_str()) {
			return Err(Error::NonEnglishInput { field: format!("$.messages[{idx}].content") });
		}
	}

	Ok(())
}

fn reject_extracted_note_if_evidence_invalid(
	cfg: &Config,
	reason: Option<&String>,
	evidence: &[EvidenceQuote],
	message_texts: &[String],
) -> Option<AddEventResult> {
	if evidence.is_empty()
		|| evidence.len() < cfg.security.evidence_min_quotes as usize
		|| evidence.len() > cfg.security.evidence_max_quotes as usize
	{
		return Some(AddEventResult {
			note_id: None,
			op: NoteOp::Rejected,
			policy_decision: MemoryPolicyDecision::Reject,
			reason_code: Some(REJECT_EVIDENCE_MISMATCH.to_string()),
			reason: reason.cloned(),
			field_path: None,
		});
	}

	for quote in evidence {
		if quote.quote.len() > cfg.security.evidence_max_quote_chars as usize {
			return Some(AddEventResult {
				note_id: None,
				op: NoteOp::Rejected,
				policy_decision: MemoryPolicyDecision::Reject,
				reason_code: Some(REJECT_EVIDENCE_MISMATCH.to_string()),
				reason: reason.cloned(),
				field_path: None,
			});
		}
		if !evidence::evidence_matches(message_texts, quote.message_index, quote.quote.as_str()) {
			return Some(AddEventResult {
				note_id: None,
				op: NoteOp::Rejected,
				policy_decision: MemoryPolicyDecision::Reject,
				reason_code: Some(REJECT_EVIDENCE_MISMATCH.to_string()),
				reason: reason.cloned(),
				field_path: None,
			});
		}
	}

	None
}

fn reject_extracted_note_if_structured_invalid(
	structured: Option<&StructuredFields>,
	text: &str,
	evidence: &[EvidenceQuote],
	reason: Option<&String>,
) -> Option<AddEventResult> {
	let structured = structured?;

	if structured.is_effectively_empty() {
		return None;
	}

	let event_evidence: Vec<(usize, String)> =
		evidence.iter().map(|q| (q.message_index, q.quote.clone())).collect();

	if let Err(err) = crate::structured_fields::validate_structured_fields(
		structured,
		text,
		&serde_json::json!({}),
		Some(event_evidence.as_slice()),
	) {
		tracing::info!(error = %err, "Rejecting extracted note due to invalid structured fields.");

		let field_path = extract_structured_rejection_field_path(&err);

		return Some(AddEventResult {
			note_id: None,
			op: NoteOp::Rejected,
			policy_decision: MemoryPolicyDecision::Reject,
			reason_code: Some(REJECT_STRUCTURED_INVALID.to_string()),
			reason: reason.cloned(),
			field_path,
		});
	}

	None
}

fn reject_extracted_note_if_writegate_rejects(
	cfg: &Config,
	reason: Option<&String>,
	note_type: &str,
	scope: &str,
	text: &str,
) -> Option<AddEventResult> {
	let gate_input = elf_domain::writegate::NoteInput {
		note_type: note_type.to_string(),
		scope: scope.to_string(),
		text: text.to_string(),
	};

	if let Err(code) = elf_domain::writegate::writegate(&gate_input, cfg) {
		return Some(AddEventResult {
			note_id: None,
			op: NoteOp::Rejected,
			policy_decision: MemoryPolicyDecision::Reject,
			reason_code: Some(crate::writegate_reason_code(code).to_string()),
			reason: reason.cloned(),
			field_path: None,
		});
	}

	None
}

fn extract_structured_rejection_field_path(err: &Error) -> Option<String> {
	match err {
		Error::NonEnglishInput { field } => Some(field.clone()),
		Error::InvalidRequest { message } if message.starts_with("structured.") =>
			message.split_whitespace().next().map(ToString::to_string),
		_ => None,
	}
}

fn build_extractor_messages(
	messages: &[EventMessage],
	max_notes: u32,
	max_note_chars: u32,
) -> Result<Vec<Value>> {
	let schema = serde_json::json!({
		"notes": [
			{
				"type": "preference|constraint|decision|profile|fact|plan",
				"key": "string|null",
				"text": "English-only sentence <= MAX_NOTE_CHARS",
				"structured": {
					"summary": "string|null",
					"facts": "string[]|null",
					"concepts": "string[]|null",
					"entities": [
						{
							"canonical": "string|null",
							"kind": "string|null",
							"aliases": "string[]|null"
						}
					],
					"relations": [
						{
							"subject": {
								"canonical": "string|null",
								"kind": "string|null",
								"aliases": "string[]|null"
							},
							"predicate": "string",
							"object": {
								"entity": {
									"canonical": "string|null",
									"kind": "string|null",
									"aliases": "string[]|null"
								},
								"value": "string|null"
							},
							"valid_from": "string|null",
							"valid_to": "string|null"
						}
					]
				},
				"importance": 0.0,
				"confidence": 0.0,
				"ttl_days": "number|null",
				"scope_suggestion": "agent_private|project_shared|org_shared|null",
				"evidence": [
					{ "message_index": "number", "quote": "string" }
				],
				"reason": "string"
			}
		]
	});
	let system_prompt = "You are a memory extraction engine for an agent memory system. \
Output must be valid JSON only and must match the provided schema exactly. \
Extract at most MAX_NOTES high-signal, cross-session reusable memory notes from the given messages. \
Each note must be one English sentence and must not contain any CJK characters. \
The structured field is optional. If present, summary must be short, facts must be short sentences supported by the evidence quotes, and concepts must be short phrases. \
structured.entities and structured.relations should mirror the structured schema with optional entity and relation metadata and relation timestamps. \
Preserve numbers, dates, percentages, currency amounts, tickers, URLs, and code snippets exactly. \
Never store secrets or PII: API keys, tokens, private keys, seed phrases, passwords, bank IDs, personal addresses. \
For every note, provide 1 to 2 evidence quotes copied verbatim from the input messages and include the message_index. \
If you cannot provide verbatim evidence, omit the note. \
If content is ephemeral or not useful long-term, return an empty notes array.";
	let messages_json = serde_json::to_string(messages).map_err(|_| Error::InvalidRequest {
		message: "Failed to serialize messages for extractor.".to_string(),
	})?;
	let user_prompt = format!(
		"Return JSON matching this exact schema:\n{schema}\nConstraints:\n- MAX_NOTES = {max_notes}\n- MAX_NOTE_CHARS = {max_note_chars}\nHere are the messages as JSON:\n{messages_json}"
	);

	Ok(vec![
		serde_json::json!({ "role": "system", "content": system_prompt }),
		serde_json::json!({ "role": "user", "content": user_prompt }),
	])
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

#[allow(clippy::too_many_arguments)]
async fn record_ingest_decision(
	tx: &mut Transaction<'_, Postgres>,
	cfg: &Config,
	ctx: &AddEventContext<'_>,
	note: &ExtractedNote,
	note_type: &str,
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
	structured_present: bool,
	graph_present: bool,
) -> Result<()> {
	let args = crate::ingest_audit::IngestAuditArgs {
		tenant_id: ctx.tenant_id,
		project_id: ctx.project_id,
		agent_id: ctx.agent_id,
		scope: ctx.scope,
		pipeline: "add_event",
		note_type,
		note_key: note.key.as_deref(),
		note_id,
		base_decision,
		policy_decision,
		note_op,
		reason_code,
		similarity_best,
		key_match,
		matched_dup,
		dup_sim_threshold: cfg.memory.dup_sim_threshold,
		update_sim_threshold: cfg.memory.update_sim_threshold,
		confidence: note.confidence.unwrap_or(0.0),
		importance: note.importance.unwrap_or(0.0),
		structured_present,
		graph_present,
		policy_rule,
		min_confidence,
		min_importance,
		ts: ctx.now,
	};

	crate::ingest_audit::insert_ingest_decision(tx, args).await
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

async fn upsert_structured_fields_tx(
	tx: &mut Transaction<'_, Postgres>,
	structured: Option<&StructuredFields>,
	note_id: Uuid,
	now: OffsetDateTime,
) -> Result<()> {
	if let Some(structured) = structured
		&& !structured.is_effectively_empty()
	{
		crate::structured_fields::upsert_structured_fields_tx(tx, note_id, structured, now).await?;
	}

	Ok(())
}
