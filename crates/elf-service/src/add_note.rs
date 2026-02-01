use elf_domain::cjk::contains_cjk;
use elf_domain::ttl::compute_expires_at;
use elf_domain::writegate::{NoteInput, writegate};
use elf_storage::models::MemoryNote;

use crate::{
    ElfService, NoteOp, ServiceError, ServiceResult, UpdateDecision, embedding_version,
    enqueue_outbox_tx, insert_version, note_snapshot, resolve_update, writegate_reason_code,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddNoteRequest {
    pub tenant_id: String,
    pub project_id: String,
    pub agent_id: String,
    pub scope: String,
    pub notes: Vec<AddNoteInput>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddNoteInput {
    #[serde(rename = "type")]
    pub note_type: String,
    pub key: Option<String>,
    pub text: String,
    pub importance: f32,
    pub confidence: f32,
    pub ttl_days: Option<i64>,
    pub source_ref: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddNoteResult {
    pub note_id: Option<uuid::Uuid>,
    pub op: NoteOp,
    pub reason_code: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddNoteResponse {
    pub results: Vec<AddNoteResult>,
}

impl ElfService {
    pub async fn add_note(&self, req: AddNoteRequest) -> ServiceResult<AddNoteResponse> {
        if req.notes.is_empty() {
            return Err(ServiceError::InvalidRequest {
                message: "Notes list is empty.".to_string(),
            });
        }

        for (idx, note) in req.notes.iter().enumerate() {
            if contains_cjk(&note.text) {
                return Err(ServiceError::NonEnglishInput {
                    field: format!("$.notes[{idx}].text"),
                });
            }
            if let Some(key) = &note.key {
                if contains_cjk(key) {
                    return Err(ServiceError::NonEnglishInput {
                        field: format!("$.notes[{idx}].key"),
                    });
                }
            }
            if let Some(path) =
                find_cjk_path(&note.source_ref, &format!("$.notes[{idx}].source_ref"))
            {
                return Err(ServiceError::NonEnglishInput { field: path });
            }
        }

        let now = time::OffsetDateTime::now_utc();
        let embed_version = embedding_version(&self.cfg);
        let mut results = Vec::with_capacity(req.notes.len());

        for note in req.notes {
            let gate_input = NoteInput {
                note_type: note.note_type.clone(),
                scope: req.scope.clone(),
                text: note.text.clone(),
            };
            if let Err(code) = writegate(&gate_input, &self.cfg) {
                results.push(AddNoteResult {
                    note_id: None,
                    op: NoteOp::Rejected,
                    reason_code: Some(writegate_reason_code(code).to_string()),
                });
                continue;
            }

            let mut tx = self.db.pool.begin().await?;
            let decision = resolve_update(
                &mut tx,
                &self.cfg,
                &self.providers,
                &req.tenant_id,
                &req.project_id,
                &req.agent_id,
                &req.scope,
                &note.note_type,
                note.key.as_deref(),
                &note.text,
                now,
            )
            .await?;

            match decision {
                UpdateDecision::Add { note_id } => {
                    let expires_at =
                        compute_expires_at(note.ttl_days, &note.note_type, &self.cfg, now);
                    let memory_note = MemoryNote {
                        note_id,
                        tenant_id: req.tenant_id.clone(),
                        project_id: req.project_id.clone(),
                        agent_id: req.agent_id.clone(),
                        scope: req.scope.clone(),
                        r#type: note.note_type.clone(),
                        key: note.key.clone(),
                        text: note.text.clone(),
                        importance: note.importance,
                        confidence: note.confidence,
                        status: "active".to_string(),
                        created_at: now,
                        updated_at: now,
                        expires_at,
                        embedding_version: embed_version.clone(),
                        source_ref: note.source_ref.clone(),
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
                        "add_note",
                        "add_note",
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

                    results.push(AddNoteResult {
                        note_id: Some(note_id),
                        op: NoteOp::Add,
                        reason_code: None,
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

                    let expires_at = match note.ttl_days {
                        Some(ttl) => compute_expires_at(Some(ttl), &note.note_type, &self.cfg, now),
                        None => existing.expires_at,
                    };

                    let expires_match = if let Some(ttl_days) = note.ttl_days {
                        match existing.expires_at {
                            Some(existing_expires_at) => {
                                let existing_ttl = (existing_expires_at - existing.updated_at)
                                    .whole_days() as i64;
                                existing_ttl == ttl_days
                            }
                            None => false,
                        }
                    } else {
                        existing.expires_at == expires_at
                    };
                    let unchanged = existing.text == note.text
                        && (existing.importance - note.importance).abs() <= f32::EPSILON
                        && (existing.confidence - note.confidence).abs() <= f32::EPSILON
                        && expires_match
                        && existing.source_ref == note.source_ref;

                    if unchanged {
                        tx.commit().await?;
                        results.push(AddNoteResult {
                            note_id: Some(note_id),
                            op: NoteOp::None,
                            reason_code: None,
                        });
                        continue;
                    }

                    existing.text = note.text.clone();
                    existing.importance = note.importance;
                    existing.confidence = note.confidence;
                    existing.updated_at = now;
                    existing.expires_at = expires_at;
                    existing.source_ref = note.source_ref.clone();

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
                        "add_note",
                        "add_note",
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

                    results.push(AddNoteResult {
                        note_id: Some(note_id),
                        op: NoteOp::Update,
                        reason_code: None,
                    });
                }
                UpdateDecision::None { note_id } => {
                    tx.commit().await?;
                    results.push(AddNoteResult {
                        note_id: Some(note_id),
                        op: NoteOp::None,
                        reason_code: None,
                    });
                }
            }
        }

        Ok(AddNoteResponse { results })
    }
}

fn find_cjk_path(value: &serde_json::Value, path: &str) -> Option<String> {
    match value {
        serde_json::Value::String(text) => {
            if contains_cjk(text) {
                Some(path.to_string())
            } else {
                None
            }
        }
        serde_json::Value::Array(items) => {
            for (idx, item) in items.iter().enumerate() {
                let child_path = format!("{path}[{idx}]");
                if let Some(found) = find_cjk_path(item, &child_path) {
                    return Some(found);
                }
            }
            None
        }
        serde_json::Value::Object(map) => {
            for (key, value) in map.iter() {
                let child_path = format!("{path}[\"{}\"]", escape_json_path_key(key));
                if let Some(found) = find_cjk_path(value, &child_path) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

fn escape_json_path_key(key: &str) -> String {
    key.replace('\\', "\\\\").replace('"', "\\\"")
}
