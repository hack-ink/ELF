use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use elf_domain::{cjk, evidence, ttl, writegate};
use elf_storage::models::MemoryNote;

use crate::{
	ElfService, InsertVersionArgs, NoteOp, REJECT_EVIDENCE_MISMATCH, ResolveUpdateArgs,
	ServiceError, ServiceResult, UpdateDecision,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventMessage {
	pub role: String,
	pub content: String,
	pub ts: Option<String>,
	pub msg_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddEventRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: Option<String>,
	pub dry_run: Option<bool>,
	pub messages: Vec<EventMessage>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddEventResult {
	pub note_id: Option<Uuid>,
	pub op: NoteOp,
	pub reason_code: Option<String>,
	pub reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddEventResponse {
	pub extracted: Value,
	pub results: Vec<AddEventResult>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct ExtractorOutput {
	pub notes: Vec<ExtractedNote>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct ExtractedNote {
	#[serde(rename = "type")]
	pub note_type: Option<String>,
	pub key: Option<String>,
	pub text: Option<String>,
	pub importance: Option<f32>,
	pub confidence: Option<f32>,
	pub ttl_days: Option<i64>,
	pub scope_suggestion: Option<String>,
	pub evidence: Option<Vec<EvidenceQuote>>,
	pub reason: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct EvidenceQuote {
	pub message_index: usize,
	pub quote: String,
}

impl ElfService {
	pub async fn add_event(&self, req: AddEventRequest) -> ServiceResult<AddEventResponse> {
		if req.messages.is_empty() {
			return Err(ServiceError::InvalidRequest {
				message: "Messages list is empty.".to_string(),
			});
		}
		if req.tenant_id.trim().is_empty()
			|| req.project_id.trim().is_empty()
			|| req.agent_id.trim().is_empty()
		{
			return Err(ServiceError::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}
		if let Some(scope) = req.scope.as_ref()
			&& scope.trim().is_empty()
		{
			return Err(ServiceError::InvalidRequest {
				message: "scope must not be empty when provided.".to_string(),
			});
		}

		for (idx, msg) in req.messages.iter().enumerate() {
			if cjk::contains_cjk(&msg.content) {
				return Err(ServiceError::NonEnglishInput {
					field: format!("$.messages[{idx}].content"),
				});
			}
		}

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
			.map_err(|_| ServiceError::InvalidRequest {
				message: "Extractor output is missing notes array.".to_string(),
			})?;

		let max_notes = self.cfg.memory.max_notes_per_add_event as usize;
		if extracted.notes.len() > max_notes {
			extracted.notes.truncate(max_notes);
		}

		let extracted_json =
			serde_json::to_value(&extracted).map_err(|_| ServiceError::InvalidRequest {
				message: "Failed to serialize extracted notes.".to_string(),
			})?;

		let now = OffsetDateTime::now_utc();
		let embed_version = crate::embedding_version(&self.cfg);
		let dry_run = req.dry_run.unwrap_or(false);
		let mut results = Vec::with_capacity(extracted.notes.len());
		let message_texts: Vec<String> = req.messages.iter().map(|m| m.content.clone()).collect();

		for note in extracted.notes {
			let note_type = note.note_type.unwrap_or_default();
			let text = note.text.unwrap_or_default();
			let importance = note.importance.unwrap_or(0.0);
			let confidence = note.confidence.unwrap_or(0.0);
			let ttl_days = note.ttl_days;
			let scope = req.scope.clone().or(note.scope_suggestion.clone()).unwrap_or_default();
			let evidence = note.evidence.unwrap_or_default();

			if evidence.is_empty()
				|| evidence.len() < self.cfg.security.evidence_min_quotes as usize
				|| evidence.len() > self.cfg.security.evidence_max_quotes as usize
			{
				results.push(AddEventResult {
					note_id: None,
					op: NoteOp::Rejected,
					reason_code: Some(REJECT_EVIDENCE_MISMATCH.to_string()),
					reason: note.reason.clone(),
				});
				continue;
			}

			let mut evidence_ok = true;
			for quote in &evidence {
				if quote.quote.len() > self.cfg.security.evidence_max_quote_chars as usize {
					evidence_ok = false;
					break;
				}
				if !evidence::evidence_matches(&message_texts, quote.message_index, &quote.quote) {
					evidence_ok = false;
					break;
				}
			}

			if !evidence_ok {
				results.push(AddEventResult {
					note_id: None,
					op: NoteOp::Rejected,
					reason_code: Some(REJECT_EVIDENCE_MISMATCH.to_string()),
					reason: note.reason.clone(),
				});
				continue;
			}

			let gate_input = writegate::NoteInput {
				note_type: note_type.clone(),
				scope: scope.clone(),
				text: text.clone(),
			};
			if let Err(code) = writegate::writegate(&gate_input, &self.cfg) {
				results.push(AddEventResult {
					note_id: None,
					op: NoteOp::Rejected,
					reason_code: Some(crate::writegate_reason_code(code).to_string()),
					reason: note.reason.clone(),
				});
				continue;
			}

			let expires_at = ttl::compute_expires_at(ttl_days, &note_type, &self.cfg, now);
			let mut tx = self.db.pool.begin().await?;
			let decision = crate::resolve_update(
				&mut tx,
				ResolveUpdateArgs {
					cfg: &self.cfg,
					providers: &self.providers,
					tenant_id: &req.tenant_id,
					project_id: &req.project_id,
					agent_id: &req.agent_id,
					scope: &scope,
					note_type: &note_type,
					key: note.key.as_deref(),
					text: &text,
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
				results.push(AddEventResult {
					note_id,
					op,
					reason_code: None,
					reason: note.reason.clone(),
				});
				continue;
			}

			let source_ref = serde_json::json!({
				"evidence": evidence,
				"reason": note.reason.clone().unwrap_or_default(),
			});

			match decision {
				UpdateDecision::Add { note_id } => {
					let memory_note = MemoryNote {
						note_id,
						tenant_id: req.tenant_id.clone(),
						project_id: req.project_id.clone(),
						agent_id: req.agent_id.clone(),
						scope: scope.clone(),
						r#type: note_type.clone(),
						key: note.key.clone(),
						text: text.clone(),
						importance,
						confidence,
						status: "active".to_string(),
						created_at: now,
						updated_at: now,
						expires_at,
						embedding_version: embed_version.clone(),
						source_ref,
						hit_count: 0,
						last_hit_at: None,
					};

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
					.execute(&mut *tx)
					.await?;

					crate::insert_version(
						&mut tx,
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
						&mut tx,
						memory_note.note_id,
						"UPSERT",
						&memory_note.embedding_version,
						now,
					)
					.await?;
					tx.commit().await?;

					results.push(AddEventResult {
						note_id: Some(note_id),
						op: NoteOp::Add,
						reason_code: None,
						reason: note.reason.clone(),
					});
				},
				UpdateDecision::Update { note_id } => {
					let mut existing: MemoryNote = sqlx::query_as!(
						MemoryNote,
						"SELECT * FROM memory_notes WHERE note_id = $1 FOR UPDATE",
						note_id,
					)
					.fetch_one(&mut *tx)
					.await?;
					let prev_snapshot = crate::note_snapshot(&existing);

					existing.text = text.clone();
					existing.importance = importance;
					existing.confidence = confidence;
					existing.updated_at = now;
					existing.expires_at = expires_at;
					existing.source_ref = source_ref;

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
						existing.text.as_str(),
						existing.importance,
						existing.confidence,
						existing.updated_at,
						existing.expires_at,
						&existing.source_ref,
						existing.note_id,
					)
					.execute(&mut *tx)
					.await?;

					crate::insert_version(
						&mut tx,
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
						&mut tx,
						existing.note_id,
						"UPSERT",
						&existing.embedding_version,
						now,
					)
					.await?;
					tx.commit().await?;

					results.push(AddEventResult {
						note_id: Some(note_id),
						op: NoteOp::Update,
						reason_code: None,
						reason: note.reason.clone(),
					});
				},
				UpdateDecision::None { note_id } => {
					tx.commit().await?;
					results.push(AddEventResult {
						note_id: Some(note_id),
						op: NoteOp::None,
						reason_code: None,
						reason: note.reason.clone(),
					});
				},
			}
		}

		Ok(AddEventResponse { extracted: extracted_json, results })
	}
}

fn build_extractor_messages(
	messages: &[EventMessage],
	max_notes: u32,
	max_note_chars: u32,
) -> ServiceResult<Vec<Value>> {
	let schema = serde_json::json!({
		"notes": [
			{
				"type": "preference|constraint|decision|profile|fact|plan",
				"key": "string|null",
				"text": "English-only sentence <= MAX_NOTE_CHARS",
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
Preserve numbers, dates, percentages, currency amounts, tickers, URLs, and code snippets exactly. \
Never store secrets or PII: API keys, tokens, private keys, seed phrases, passwords, bank IDs, personal addresses. \
For every note, provide 1 to 2 evidence quotes copied verbatim from the input messages and include the message_index. \
If you cannot provide verbatim evidence, omit the note. \
If content is ephemeral or not useful long-term, return an empty notes array.";

	let messages_json =
		serde_json::to_string(messages).map_err(|_| ServiceError::InvalidRequest {
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
