use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	ElfService, Error, InsertVersionArgs, NoteOp, REJECT_EVIDENCE_MISMATCH, ResolveUpdateArgs,
	Result, UpdateDecision, structured_fields::StructuredFields,
};
use elf_config::Config;
use elf_domain::{cjk, evidence, ttl};
use elf_storage::models::MemoryNote;

const REJECT_STRUCTURED_INVALID: &str = "REJECT_STRUCTURED_INVALID";

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
	pub reason_code: Option<String>,
	pub reason: Option<String>,
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

struct PersistExtractedNoteArgs<'a> {
	req: &'a AddEventRequest,
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
		let now = OffsetDateTime::now_utc();
		let embed_version = crate::embedding_version(&self.cfg);
		let dry_run = req.dry_run.unwrap_or(false);
		let message_texts: Vec<String> = req.messages.iter().map(|m| m.content.clone()).collect();
		let mut results = Vec::with_capacity(extracted.notes.len());

		for note in extracted.notes {
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
		let note_type = note.r#type.clone().unwrap_or_default();
		let text = note.text.clone().unwrap_or_default();
		let structured = note.structured.clone();
		let importance = note.importance.unwrap_or(0.0);
		let confidence = note.confidence.unwrap_or(0.0);
		let ttl_days = note.ttl_days;
		let scope = req.scope.clone().or(note.scope_suggestion.clone()).unwrap_or_default();
		let evidence = note.evidence.clone().unwrap_or_default();

		if let Some(result) = reject_extracted_note_if_evidence_invalid(
			&self.cfg,
			note.reason.as_ref(),
			&evidence,
			message_texts,
		) {
			return Ok(result);
		}
		if let Some(result) = reject_extracted_note_if_structured_invalid(
			structured.as_ref(),
			text.as_str(),
			&evidence,
			note.reason.as_ref(),
		) {
			return Ok(result);
		}
		if let Some(result) = reject_extracted_note_if_writegate_rejects(
			&self.cfg,
			note.reason.as_ref(),
			&note_type,
			&scope,
			&text,
		) {
			return Ok(result);
		}

		let expires_at = ttl::compute_expires_at(ttl_days, note_type.as_str(), &self.cfg, now);
		let mut tx = self.db.pool.begin().await?;
		let decision = crate::resolve_update(
			&mut *tx,
			ResolveUpdateArgs {
				cfg: &self.cfg,
				providers: &self.providers,
				tenant_id: req.tenant_id.as_str(),
				project_id: req.project_id.as_str(),
				agent_id: req.agent_id.as_str(),
				scope: scope.as_str(),
				note_type: note_type.as_str(),
				key: note.key.as_deref(),
				text: text.as_str(),
				now,
			},
		)
		.await?;

		if dry_run {
			tx.commit().await?;

			let (note_id, op) = match decision {
				UpdateDecision::Add { note_id } => (Some(note_id), NoteOp::Add),
				UpdateDecision::Update { note_id } => (Some(note_id), NoteOp::Update),
				UpdateDecision::None { note_id } => (Some(note_id), NoteOp::None),
			};

			return Ok(AddEventResult {
				note_id,
				op,
				reason_code: None,
				reason: note.reason.clone(),
			});
		}

		let source_ref = serde_json::json!({
			"evidence": evidence,
			"reason": note.reason.clone().unwrap_or_default(),
		});
		let result = self
			.persist_extracted_note_decision(
				&mut tx,
				PersistExtractedNoteArgs {
					req,
					structured: structured.as_ref(),
					key: note.key.as_deref(),
					reason: note.reason.as_ref(),
					note_type: note_type.as_str(),
					text: text.as_str(),
					scope: scope.as_str(),
					importance,
					confidence,
					expires_at,
					source_ref,
					now,
					embed_version,
				},
				decision,
			)
			.await?;

		tx.commit().await?;

		Ok(result)
	}

	async fn persist_extracted_note_decision(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		args: PersistExtractedNoteArgs<'_>,
		decision: UpdateDecision,
	) -> Result<AddEventResult> {
		match (decision, args) {
			(UpdateDecision::Add { note_id }, args) =>
				self.persist_extracted_note_add(tx, args, note_id).await,
			(UpdateDecision::Update { note_id }, args) =>
				self.persist_extracted_note_update(tx, args, note_id).await,
			(UpdateDecision::None { note_id }, args) =>
				self.persist_extracted_note_none(tx, args, note_id).await,
		}
	}

	async fn persist_extracted_note_add(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		args: PersistExtractedNoteArgs<'_>,
		note_id: Uuid,
	) -> Result<AddEventResult> {
		let memory_note = MemoryNote {
			note_id,
			tenant_id: args.req.tenant_id.clone(),
			project_id: args.req.project_id.clone(),
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

		Ok(AddEventResult {
			note_id: Some(note_id),
			op: NoteOp::Add,
			reason_code: None,
			reason: args.reason.cloned(),
		})
	}

	async fn persist_extracted_note_update(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		args: PersistExtractedNoteArgs<'_>,
		note_id: Uuid,
	) -> Result<AddEventResult> {
		let mut existing: MemoryNote = sqlx::query_as!(
			MemoryNote,
			"SELECT * FROM memory_notes WHERE note_id = $1 FOR UPDATE",
			note_id,
		)
		.fetch_one(&mut **tx)
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

		Ok(AddEventResult {
			note_id: Some(note_id),
			op: NoteOp::Update,
			reason_code: None,
			reason: args.reason.cloned(),
		})
	}

	async fn persist_extracted_note_none(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		args: PersistExtractedNoteArgs<'_>,
		note_id: Uuid,
	) -> Result<AddEventResult> {
		if let Some(structured) = args.structured
			&& !structured.is_effectively_empty()
		{
			crate::structured_fields::upsert_structured_fields_tx(
				tx, note_id, structured, args.now,
			)
			.await?;
			crate::enqueue_outbox_tx(&mut **tx, note_id, "UPSERT", args.embed_version, args.now)
				.await?;

			return Ok(AddEventResult {
				note_id: Some(note_id),
				op: NoteOp::Update,
				reason_code: None,
				reason: args.reason.cloned(),
			});
		}

		Ok(AddEventResult {
			note_id: Some(note_id),
			op: NoteOp::None,
			reason_code: None,
			reason: args.reason.cloned(),
		})
	}
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
			reason_code: Some(REJECT_EVIDENCE_MISMATCH.to_string()),
			reason: reason.cloned(),
		});
	}

	for quote in evidence {
		if quote.quote.len() > cfg.security.evidence_max_quote_chars as usize {
			return Some(AddEventResult {
				note_id: None,
				op: NoteOp::Rejected,
				reason_code: Some(REJECT_EVIDENCE_MISMATCH.to_string()),
				reason: reason.cloned(),
			});
		}
		if !evidence::evidence_matches(message_texts, quote.message_index, quote.quote.as_str()) {
			return Some(AddEventResult {
				note_id: None,
				op: NoteOp::Rejected,
				reason_code: Some(REJECT_EVIDENCE_MISMATCH.to_string()),
				reason: reason.cloned(),
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

		return Some(AddEventResult {
			note_id: None,
			op: NoteOp::Rejected,
			reason_code: Some(REJECT_STRUCTURED_INVALID.to_string()),
			reason: reason.cloned(),
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
			reason_code: Some(crate::writegate_reason_code(code).to_string()),
			reason: reason.cloned(),
		});
	}

	None
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
					"concepts": "string[]|null"
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

async fn insert_memory_note_tx(
	tx: &mut Transaction<'_, Postgres>,
	memory_note: &MemoryNote,
) -> Result<()> {
	sqlx::query!(
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
		memory_note.note_id,
		memory_note.tenant_id.as_str(),
		memory_note.project_id.as_str(),
		memory_note.agent_id.as_str(),
		memory_note.scope.as_str(),
		memory_note.r#type.as_str(),
		memory_note.key.as_deref(),
		memory_note.text.as_str(),
		memory_note.importance,
		memory_note.confidence,
		memory_note.status.as_str(),
		memory_note.created_at,
		memory_note.updated_at,
		memory_note.expires_at,
		memory_note.embedding_version.as_str(),
		&memory_note.source_ref,
		memory_note.hit_count,
		memory_note.last_hit_at,
	)
	.execute(&mut **tx)
	.await?;

	Ok(())
}

async fn update_memory_note_tx(
	tx: &mut Transaction<'_, Postgres>,
	memory_note: &MemoryNote,
) -> Result<()> {
	sqlx::query!(
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
		memory_note.text.as_str(),
		memory_note.importance,
		memory_note.confidence,
		memory_note.updated_at,
		memory_note.expires_at,
		&memory_note.source_ref,
		memory_note.note_id,
	)
	.execute(&mut **tx)
	.await?;

	Ok(())
}
