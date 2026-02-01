use elf_domain::cjk::contains_cjk;
use elf_domain::evidence::evidence_matches;
use elf_domain::ttl::compute_expires_at;
use elf_domain::writegate::{NoteInput, writegate};
use elf_storage::models::MemoryNote;

use crate::{
    ElfService, NoteOp, REJECT_EVIDENCE_MISMATCH, ServiceError, ServiceResult, UpdateDecision,
    embedding_version, enqueue_outbox_tx, insert_version, note_snapshot, resolve_update,
    writegate_reason_code,
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
    pub note_id: Option<uuid::Uuid>,
    pub op: NoteOp,
    pub reason_code: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddEventResponse {
    pub extracted: serde_json::Value,
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

        for (idx, msg) in req.messages.iter().enumerate() {
            if contains_cjk(&msg.content) {
                return Err(ServiceError::NonEnglishInput {
                    field: format!("messages[{idx}].content"),
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

        let extracted_json = serde_json::to_value(&extracted).map_err(|_| ServiceError::InvalidRequest {
            message: "Failed to serialize extracted notes.".to_string(),
        })?;

        let now = time::OffsetDateTime::now_utc();
        let embed_version = embedding_version(&self.cfg);
        let dry_run = req.dry_run.unwrap_or(false);
        let mut results = Vec::with_capacity(extracted.notes.len());

        for note in extracted.notes {
            let note_type = note.note_type.unwrap_or_default();
            let text = note.text.unwrap_or_default();
            let importance = note.importance.unwrap_or(0.0);
            let confidence = note.confidence.unwrap_or(0.0);
            let ttl_days = note.ttl_days;
            let scope = req
                .scope
                .clone()
                .or(note.scope_suggestion.clone())
                .unwrap_or_default();
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
                if !evidence_matches(
                    &req.messages.iter().map(|m| m.content.clone()).collect::<Vec<_>>(),
                    quote.message_index,
                    &quote.quote,
                ) {
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

            let gate_input = NoteInput {
                note_type: note_type.clone(),
                scope: scope.clone(),
                text: text.clone(),
            };
            if let Err(code) = writegate(&gate_input, &self.cfg) {
                results.push(AddEventResult {
                    note_id: None,
                    op: NoteOp::Rejected,
                    reason_code: Some(writegate_reason_code(code).to_string()),
                    reason: note.reason.clone(),
                });
                continue;
            }

            let expires_at = compute_expires_at(ttl_days, &note_type, &self.cfg, now);
            let mut tx = self.db.pool.begin().await?;
            let decision = resolve_update(
                &mut tx,
                &self.cfg,
                &self.providers,
                &req.tenant_id,
                &req.project_id,
                &req.agent_id,
                &scope,
                &note_type,
                note.key.as_deref(),
                &text,
                now,
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

                    sqlx::query(
                        "INSERT INTO memory_notes \
                         (note_id, tenant_id, project_id, agent_id, scope, type, key, text, importance, confidence, status, created_at, updated_at, expires_at, embedding_version, source_ref, hit_count, last_hit_at) \
                         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)",
                    )
                    .bind(memory_note.note_id)
                    .bind(&memory_note.tenant_id)
                    .bind(&memory_note.project_id)
                    .bind(&memory_note.agent_id)
                    .bind(&memory_note.scope)
                    .bind(&memory_note.r#type)
                    .bind(&memory_note.key)
                    .bind(&memory_note.text)
                    .bind(memory_note.importance)
                    .bind(memory_note.confidence)
                    .bind(&memory_note.status)
                    .bind(memory_note.created_at)
                    .bind(memory_note.updated_at)
                    .bind(memory_note.expires_at)
                    .bind(&memory_note.embedding_version)
                    .bind(&memory_note.source_ref)
                    .bind(memory_note.hit_count)
                    .bind(memory_note.last_hit_at)
                    .execute(&mut *tx)
                    .await?;

                    insert_version(
                        &mut tx,
                        memory_note.note_id,
                        "ADD",
                        None,
                        Some(note_snapshot(&memory_note)),
                        "add_event",
                        "add_event",
                        now,
                    )
                    .await?;
                    enqueue_outbox_tx(
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
                }
                UpdateDecision::Update { note_id } => {
                    let mut existing: MemoryNote = sqlx::query_as(
                        "SELECT * FROM memory_notes WHERE note_id = $1 FOR UPDATE",
                    )
                    .bind(note_id)
                    .fetch_one(&mut *tx)
                    .await?;
                    let prev_snapshot = note_snapshot(&existing);

                    existing.text = text.clone();
                    existing.importance = importance;
                    existing.confidence = confidence;
                    existing.updated_at = now;
                    existing.expires_at = expires_at;
                    existing.source_ref = source_ref;

                    sqlx::query(
                        "UPDATE memory_notes SET text = $1, importance = $2, confidence = $3, updated_at = $4, expires_at = $5, source_ref = $6 WHERE note_id = $7",
                    )
                    .bind(&existing.text)
                    .bind(existing.importance)
                    .bind(existing.confidence)
                    .bind(existing.updated_at)
                    .bind(existing.expires_at)
                    .bind(&existing.source_ref)
                    .bind(existing.note_id)
                    .execute(&mut *tx)
                    .await?;

                    insert_version(
                        &mut tx,
                        existing.note_id,
                        "UPDATE",
                        Some(prev_snapshot),
                        Some(note_snapshot(&existing)),
                        "add_event",
                        "add_event",
                        now,
                    )
                    .await?;
                    enqueue_outbox_tx(
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
                }
                UpdateDecision::None { note_id } => {
                    tx.commit().await?;
                    results.push(AddEventResult {
                        note_id: Some(note_id),
                        op: NoteOp::None,
                        reason_code: None,
                        reason: note.reason.clone(),
                    });
                }
            }
        }

        Ok(AddEventResponse {
            extracted: extracted_json,
            results,
        })
    }
}

fn build_extractor_messages(
    messages: &[EventMessage],
    max_notes: u32,
    max_note_chars: u32,
) -> ServiceResult<Vec<serde_json::Value>> {
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

    let messages_json = serde_json::to_string(messages).map_err(|_| ServiceError::InvalidRequest {
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
