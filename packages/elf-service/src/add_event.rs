use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use elf_domain::{cjk, evidence, ttl, writegate};
use elf_storage::models::MemoryNote;

use crate::{
	ElfService, Error, InsertVersionArgs, NoteOp, REJECT_EVIDENCE_MISMATCH, ResolveUpdateArgs,
	Result, UpdateDecision,
	structured_fields::{
		StructuredFields, upsert_structured_fields_tx, validate_structured_fields,
	},
};

type PgTx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

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

#[derive(Clone, Debug)]
struct PreparedEventNote {
	note_type: String,
	key: Option<String>,
	text: String,
	structured: Option<StructuredFields>,
	importance: f32,
	confidence: f32,
	ttl_days: Option<i64>,
	scope: String,
	evidence: Vec<EvidenceQuote>,
	reason: Option<String>,
}
impl PreparedEventNote {
	fn from_extracted(note: ExtractedNote, request_scope: Option<String>) -> Self {
		let ExtractedNote {
			r#type,
			key,
			text,
			structured,
			importance,
			confidence,
			ttl_days,
			scope_suggestion,
			evidence,
			reason,
		} = note;

		Self {
			note_type: r#type.unwrap_or_default(),
			key,
			text: text.unwrap_or_default(),
			structured,
			importance: importance.unwrap_or(0.0),
			confidence: confidence.unwrap_or(0.0),
			ttl_days,
			scope: request_scope.or(scope_suggestion).unwrap_or_default(),
			evidence: evidence.unwrap_or_default(),
			reason,
		}
	}
}

impl ElfService {
	pub async fn add_event(&self, req: AddEventRequest) -> Result<AddEventResponse> {
		validate_add_event_request(&req)?;

		let (notes, extracted_json) = self.extract_add_event_notes(&req).await?;
		let now = OffsetDateTime::now_utc();
		let embed_version = crate::embedding_version(&self.cfg);
		let dry_run = req.dry_run.unwrap_or(false);
		let message_texts: Vec<String> = req.messages.iter().map(|m| m.content.clone()).collect();
		let results = self
			.process_extracted_notes(
				&req,
				notes,
				now,
				embed_version.as_str(),
				dry_run,
				&message_texts,
			)
			.await?;

		Ok(AddEventResponse { extracted: extracted_json, results })
	}

	async fn extract_add_event_notes(
		&self,
		req: &AddEventRequest,
	) -> Result<(Vec<ExtractedNote>, Value)> {
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
		let mut extracted: ExtractorOutput = serde_json::from_value(extracted_raw.clone())
			.map_err(|_| Error::InvalidRequest {
				message: "Extractor output is missing notes array.".to_string(),
			})?;
		let max_notes = self.cfg.memory.max_notes_per_add_event as usize;

		if extracted.notes.len() > max_notes {
			extracted.notes.truncate(max_notes);
		}

		let extracted_json = serde_json::to_value(&extracted).map_err(|_| {
			Error::InvalidRequest { message: "Failed to serialize extracted notes.".to_string() }
		})?;

		Ok((extracted.notes, extracted_json))
	}

	async fn process_extracted_notes(
		&self,
		req: &AddEventRequest,
		notes: Vec<ExtractedNote>,
		now: OffsetDateTime,
		embed_version: &str,
		dry_run: bool,
		message_texts: &[String],
	) -> Result<Vec<AddEventResult>> {
		let mut results = Vec::with_capacity(notes.len());

		for note in notes {
			let result = self
				.process_extracted_note(req, note, now, embed_version, dry_run, message_texts)
				.await?;

			results.push(result);
		}

		Ok(results)
	}

	async fn process_extracted_note(
		&self,
		req: &AddEventRequest,
		note: ExtractedNote,
		now: OffsetDateTime,
		embed_version: &str,
		dry_run: bool,
		message_texts: &[String],
	) -> Result<AddEventResult> {
		let note = PreparedEventNote::from_extracted(note, req.scope.clone());

		if !self.has_valid_event_evidence(&note.evidence, message_texts) {
			return Ok(rejected_result(REJECT_EVIDENCE_MISMATCH, note.reason.clone()));
		}
		if !validate_event_structured_fields(&note) {
			return Ok(rejected_result(REJECT_STRUCTURED_INVALID, note.reason.clone()));
		}

		let gate_input = writegate::NoteInput {
			note_type: note.note_type.clone(),
			scope: note.scope.clone(),
			text: note.text.clone(),
		};

		if let Err(code) = writegate::writegate(&gate_input, &self.cfg) {
			return Ok(rejected_result(crate::writegate_reason_code(code), note.reason.clone()));
		}

		let expires_at = ttl::compute_expires_at(note.ttl_days, &note.note_type, &self.cfg, now);
		let mut tx = self.db.pool.begin().await?;
		let decision = crate::resolve_update(
			&mut *tx,
			ResolveUpdateArgs {
				cfg: &self.cfg,
				providers: &self.providers,
				tenant_id: &req.tenant_id,
				project_id: &req.project_id,
				agent_id: &req.agent_id,
				scope: &note.scope,
				note_type: &note.note_type,
				key: note.key.as_deref(),
				text: &note.text,
				now,
			},
		)
		.await?;

		if dry_run {
			tx.commit().await?;

			return Ok(dry_run_result(decision, note.reason.clone()));
		}

		let source_ref = serde_json::json!({
			"evidence": note.evidence,
			"reason": note.reason.clone().unwrap_or_default(),
		});

		self.apply_decision(
			&mut tx,
			decision,
			req,
			&note,
			now,
			expires_at,
			embed_version,
			source_ref,
		)
		.await
	}

	fn has_valid_event_evidence(
		&self,
		evidence: &[EvidenceQuote],
		message_texts: &[String],
	) -> bool {
		if evidence.is_empty()
			|| evidence.len() < self.cfg.security.evidence_min_quotes as usize
			|| evidence.len() > self.cfg.security.evidence_max_quotes as usize
		{
			return false;
		}

		for quote in evidence {
			if quote.quote.len() > self.cfg.security.evidence_max_quote_chars as usize {
				return false;
			}
			if !evidence::evidence_matches(message_texts, quote.message_index, &quote.quote) {
				return false;
			}
		}

		true
	}

	async fn apply_decision(
		&self,
		tx: &mut PgTx<'_>,
		decision: UpdateDecision,
		req: &AddEventRequest,
		note: &PreparedEventNote,
		now: OffsetDateTime,
		expires_at: Option<OffsetDateTime>,
		embed_version: &str,
		source_ref: Value,
	) -> Result<AddEventResult> {
		match decision {
			UpdateDecision::Add { note_id } =>
				self.persist_add(tx, req, note, note_id, now, expires_at, embed_version, source_ref)
					.await,
			UpdateDecision::Update { note_id } =>
				self.persist_update(tx, note, note_id, now, expires_at, source_ref).await,
			UpdateDecision::None { note_id } =>
				self.persist_none(tx, note, note_id, now, embed_version).await,
		}
	}

	async fn persist_add(
		&self,
		tx: &mut PgTx<'_>,
		req: &AddEventRequest,
		note: &PreparedEventNote,
		note_id: Uuid,
		now: OffsetDateTime,
		expires_at: Option<OffsetDateTime>,
		embed_version: &str,
		source_ref: Value,
	) -> Result<AddEventResult> {
		let memory_note = MemoryNote {
			note_id,
			tenant_id: req.tenant_id.clone(),
			project_id: req.project_id.clone(),
			agent_id: req.agent_id.clone(),
			scope: note.scope.clone(),
			r#type: note.note_type.clone(),
			key: note.key.clone(),
			text: note.text.clone(),
			importance: note.importance,
			confidence: note.confidence,
			status: "active".to_string(),
			created_at: now,
			updated_at: now,
			expires_at,
			embedding_version: embed_version.to_string(),
			source_ref,
			hit_count: 0,
			last_hit_at: None,
		};

		sqlx::query!(
			"INSERT INTO memory_notes (note_id, tenant_id, project_id, agent_id, scope, type, key, text, importance, confidence, status, created_at, updated_at, expires_at, embedding_version, source_ref, hit_count, last_hit_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)",
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

		crate::insert_version(
			&mut **tx,
			InsertVersionArgs {
				note_id: memory_note.note_id,
				op: "ADD",
				prev_snapshot: None,
				new_snapshot: Some(crate::note_snapshot(&memory_note)),
				reason: "add_event",
				actor: "add_event",
				ts: now,
			},
		)
		.await?;
		crate::enqueue_outbox_tx(
			&mut **tx,
			memory_note.note_id,
			"UPSERT",
			&memory_note.embedding_version,
			now,
		)
		.await?;

		self.upsert_structured_if_present(tx, memory_note.note_id, note.structured.as_ref(), now)
			.await?;
		tx.commit().await?;

		Ok(AddEventResult {
			note_id: Some(note_id),
			op: NoteOp::Add,
			reason_code: None,
			reason: note.reason.clone(),
		})
	}

	async fn persist_update(
		&self,
		tx: &mut PgTx<'_>,
		note: &PreparedEventNote,
		note_id: Uuid,
		now: OffsetDateTime,
		expires_at: Option<OffsetDateTime>,
		source_ref: Value,
	) -> Result<AddEventResult> {
		let mut existing: MemoryNote = sqlx::query_as!(
			MemoryNote,
			"SELECT * FROM memory_notes WHERE note_id = $1 FOR UPDATE",
			note_id
		)
		.fetch_one(&mut **tx)
		.await?;
		let prev_snapshot = crate::note_snapshot(&existing);

		existing.text = note.text.clone();
		existing.importance = note.importance;
		existing.confidence = note.confidence;
		existing.updated_at = now;
		existing.expires_at = expires_at;
		existing.source_ref = source_ref;

		sqlx::query!(
			"UPDATE memory_notes SET text = $1, importance = $2, confidence = $3, updated_at = $4, expires_at = $5, source_ref = $6 WHERE note_id = $7",
			existing.text.as_str(),
			existing.importance,
			existing.confidence,
			existing.updated_at,
			existing.expires_at,
			&existing.source_ref,
			existing.note_id,
		)
		.execute(&mut **tx)
		.await?;

		crate::insert_version(
			&mut **tx,
			InsertVersionArgs {
				note_id: existing.note_id,
				op: "UPDATE",
				prev_snapshot: Some(prev_snapshot),
				new_snapshot: Some(crate::note_snapshot(&existing)),
				reason: "add_event",
				actor: "add_event",
				ts: now,
			},
		)
		.await?;
		crate::enqueue_outbox_tx(
			&mut **tx,
			existing.note_id,
			"UPSERT",
			&existing.embedding_version,
			now,
		)
		.await?;

		self.upsert_structured_if_present(tx, existing.note_id, note.structured.as_ref(), now)
			.await?;
		tx.commit().await?;

		Ok(AddEventResult {
			note_id: Some(note_id),
			op: NoteOp::Update,
			reason_code: None,
			reason: note.reason.clone(),
		})
	}

	async fn persist_none(
		&self,
		tx: &mut PgTx<'_>,
		note: &PreparedEventNote,
		note_id: Uuid,
		now: OffsetDateTime,
		embed_version: &str,
	) -> Result<AddEventResult> {
		let structured_upserted =
			self.upsert_structured_if_present(tx, note_id, note.structured.as_ref(), now).await?;

		if structured_upserted {
			crate::enqueue_outbox_tx(&mut **tx, note_id, "UPSERT", embed_version, now).await?;

			tx.commit().await?;

			return Ok(AddEventResult {
				note_id: Some(note_id),
				op: NoteOp::Update,
				reason_code: None,
				reason: note.reason.clone(),
			});
		}

		tx.commit().await?;

		Ok(AddEventResult {
			note_id: Some(note_id),
			op: NoteOp::None,
			reason_code: None,
			reason: note.reason.clone(),
		})
	}

	async fn upsert_structured_if_present(
		&self,
		tx: &mut PgTx<'_>,
		note_id: Uuid,
		structured: Option<&StructuredFields>,
		now: OffsetDateTime,
	) -> Result<bool> {
		if let Some(structured) = structured
			&& !structured.is_effectively_empty()
		{
			upsert_structured_fields_tx(&mut **tx, note_id, structured, now).await?;

			return Ok(true);
		}

		Ok(false)
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
		if cjk::contains_cjk(&msg.content) {
			return Err(Error::NonEnglishInput { field: format!("$.messages[{idx}].content") });
		}
	}

	Ok(())
}

fn validate_event_structured_fields(note: &PreparedEventNote) -> bool {
	if let Some(structured) = note.structured.as_ref()
		&& !structured.is_effectively_empty()
	{
		let event_evidence: Vec<(usize, String)> =
			note.evidence.iter().map(|q| (q.message_index, q.quote.clone())).collect();

		if let Err(err) = validate_structured_fields(
			structured,
			&note.text,
			&serde_json::json!({}),
			Some(event_evidence.as_slice()),
		) {
			tracing::info!(error = %err, "Rejecting extracted note due to invalid structured fields.");

			return false;
		}
	}

	true
}

fn dry_run_result(decision: UpdateDecision, reason: Option<String>) -> AddEventResult {
	let (note_id, op) = match decision {
		UpdateDecision::Add { note_id } => (Some(note_id), NoteOp::Add),
		UpdateDecision::Update { note_id } => (Some(note_id), NoteOp::Update),
		UpdateDecision::None { note_id } => (Some(note_id), NoteOp::None),
	};

	AddEventResult { note_id, op, reason_code: None, reason }
}

fn rejected_result(reason_code: impl Into<String>, reason: Option<String>) -> AddEventResult {
	AddEventResult {
		note_id: None,
		op: NoteOp::Rejected,
		reason_code: Some(reason_code.into()),
		reason,
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
