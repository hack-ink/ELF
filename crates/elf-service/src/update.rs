use elf_domain::cjk::contains_cjk;
use elf_domain::ttl::compute_expires_at;
use elf_domain::writegate::{NoteInput, writegate};
use elf_storage::models::MemoryNote;

use crate::{
    ElfService, NoteOp, ServiceError, ServiceResult, enqueue_outbox_tx, insert_version,
    note_snapshot, writegate_reason_code,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateRequest {
    pub note_id: uuid::Uuid,
    pub text: Option<String>,
    pub importance: Option<f32>,
    pub confidence: Option<f32>,
    pub ttl_days: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateResponse {
    pub note_id: uuid::Uuid,
    pub op: NoteOp,
    pub reason_code: Option<String>,
}

impl ElfService {
    pub async fn update(&self, req: UpdateRequest) -> ServiceResult<UpdateResponse> {
        // TODO: Enforce tenant/project/agent ownership once update requests include namespace identifiers.
        let text_update = req.text.clone();
        let mut tx = self.db.pool.begin().await?;
        let mut note: MemoryNote = sqlx::query_as(
            "SELECT * FROM memory_notes WHERE note_id = $1 FOR UPDATE",
        )
        .bind(req.note_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ServiceError::InvalidRequest {
            message: "Note not found.".to_string(),
        })?;

        let prev_snapshot = note_snapshot(&note);

        let candidate_text = if let Some(text) = text_update.as_ref() {
            if contains_cjk(text) {
                return Err(ServiceError::NonEnglishInput {
                    field: "$.text".to_string(),
                });
            }
            text.clone()
        } else {
            note.text.clone()
        };

        let gate = NoteInput {
            note_type: note.r#type.clone(),
            scope: note.scope.clone(),
            text: candidate_text,
        };
        if let Err(code) = writegate(&gate, &self.cfg) {
            return Ok(UpdateResponse {
                note_id: note.note_id,
                op: NoteOp::Rejected,
                reason_code: Some(writegate_reason_code(code).to_string()),
            });
        }

        let mut changed = false;
        if let Some(text) = text_update {
            if text != note.text {
                note.text = text;
                changed = true;
            }
        }
        if let Some(importance) = req.importance {
            if (importance - note.importance).abs() > f32::EPSILON {
                note.importance = importance;
                changed = true;
            }
        }
        if let Some(confidence) = req.confidence {
            if (confidence - note.confidence).abs() > f32::EPSILON {
                note.confidence = confidence;
                changed = true;
            }
        }
        let now = time::OffsetDateTime::now_utc();
        if let Some(ttl_days) = req.ttl_days {
            let effective_ttl = if ttl_days > 0 {
                Some(ttl_days)
            } else {
                default_ttl_days(&note.r#type, &self.cfg)
            };

            if let Some(ttl) = effective_ttl.filter(|value| *value > 0) {
                let existing_ttl = note.expires_at.map(|expires_at| {
                    (expires_at - note.updated_at).whole_days() as i64
                });
                if existing_ttl != Some(ttl) {
                    note.expires_at =
                        compute_expires_at(Some(ttl), &note.r#type, &self.cfg, now);
                    changed = true;
                }
            } else if note.expires_at.is_some() {
                note.expires_at = None;
                changed = true;
            }
        }

        if !changed {
            tx.commit().await?;
            return Ok(UpdateResponse {
                note_id: note.note_id,
                op: NoteOp::None,
                reason_code: None,
            });
        }

        note.updated_at = now;

        sqlx::query(
            "UPDATE memory_notes SET text = $1, importance = $2, confidence = $3, updated_at = $4, expires_at = $5 WHERE note_id = $6",
        )
        .bind(&note.text)
        .bind(note.importance)
        .bind(note.confidence)
        .bind(note.updated_at)
        .bind(note.expires_at)
        .bind(note.note_id)
        .execute(&mut *tx)
        .await?;

        insert_version(
            &mut tx,
            note.note_id,
            "UPDATE",
            Some(prev_snapshot),
            Some(note_snapshot(&note)),
            "update",
            "update",
            note.updated_at,
        )
        .await?;

        enqueue_outbox_tx(
            &mut tx,
            note.note_id,
            "UPSERT",
            &note.embedding_version,
            note.updated_at,
        )
        .await?;

        tx.commit().await?;

        Ok(UpdateResponse {
            note_id: note.note_id,
            op: NoteOp::Update,
            reason_code: None,
        })
    }
}

fn default_ttl_days(note_type: &str, cfg: &elf_config::Config) -> Option<i64> {
    let days = match note_type {
        "plan" => cfg.lifecycle.ttl_days.plan,
        "fact" => cfg.lifecycle.ttl_days.fact,
        "preference" => cfg.lifecycle.ttl_days.preference,
        "constraint" => cfg.lifecycle.ttl_days.constraint,
        "decision" => cfg.lifecycle.ttl_days.decision,
        "profile" => cfg.lifecycle.ttl_days.profile,
        _ => 0,
    };
    if days > 0 { Some(days) } else { None }
}
