use std::hash::Hasher;

use elf_domain::cjk::contains_cjk;
use elf_storage::models::MemoryNote;
use qdrant_client::qdrant::{Condition, Filter, SearchPointsBuilder};
use qdrant_client::qdrant::point_id::PointIdOptions;

use crate::{ElfService, ServiceError, ServiceResult};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchRequest {
    pub tenant_id: String,
    pub project_id: String,
    pub agent_id: String,
    pub read_profile: String,
    pub query: String,
    pub top_k: Option<u32>,
    pub candidate_k: Option<u32>,
    pub record_hits: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchItem {
    pub note_id: uuid::Uuid,
    #[serde(rename = "type")]
    pub note_type: String,
    pub key: Option<String>,
    pub scope: String,
    pub text: String,
    pub importance: f32,
    pub confidence: f32,
    pub updated_at: time::OffsetDateTime,
    pub expires_at: Option<time::OffsetDateTime>,
    pub final_score: f32,
    pub source_ref: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResponse {
    pub items: Vec<SearchItem>,
}

impl ElfService {
    pub async fn search(&self, req: SearchRequest) -> ServiceResult<SearchResponse> {
        if contains_cjk(&req.query) {
            return Err(ServiceError::NonEnglishInput {
                field: "query".to_string(),
            });
        }

        let allowed_scopes = resolve_scopes(&self.cfg, &req.read_profile)?;
        if allowed_scopes.is_empty() {
            return Ok(SearchResponse { items: Vec::new() });
        }

        let top_k = req.top_k.unwrap_or(self.cfg.memory.top_k).max(1);
        let candidate_k = req
            .candidate_k
            .unwrap_or(self.cfg.memory.candidate_k)
            .max(top_k);

        let embeddings = self
            .providers
            .embedding
            .embed(&self.cfg.providers.embedding, &[req.query.clone()])
            .await?;
        let query_vec = embeddings
            .into_iter()
            .next()
            .ok_or_else(|| ServiceError::Provider {
                message: "Embedding provider returned no vectors.".to_string(),
            })?;
        if query_vec.len() != self.cfg.storage.qdrant.vector_dim as usize {
            return Err(ServiceError::Provider {
                message: "Embedding vector dimension mismatch.".to_string(),
            });
        }

        let filter = Filter::must([
            Condition::matches("tenant_id", req.tenant_id.clone()),
            Condition::matches("project_id", req.project_id.clone()),
            Condition::matches("scope", allowed_scopes.clone()),
            Condition::matches("status", "active"),
        ]);

        let search = SearchPointsBuilder::new(
            self.qdrant.collection.clone(),
            query_vec,
            candidate_k as u64,
        )
        .filter(filter);

        let search_response = self
            .qdrant
            .client
            .search_points(search)
            .await
            .map_err(|err| ServiceError::Qdrant {
                message: err.to_string(),
            })?;

        let candidate_ids: Vec<uuid::Uuid> = search_response
            .result
            .iter()
            .filter_map(|point| point.id.as_ref())
            .filter_map(point_id_to_uuid)
            .collect();

        if candidate_ids.is_empty() {
            return Ok(SearchResponse { items: Vec::new() });
        }

        let mut notes: Vec<MemoryNote> = sqlx::query_as(
            "SELECT * FROM memory_notes WHERE note_id = ANY($1)",
        )
        .bind(&candidate_ids)
        .fetch_all(&self.db.pool)
        .await?;

        let now = time::OffsetDateTime::now_utc();
        notes.retain(|note| {
            note.status == "active"
                && allowed_scopes.contains(&note.scope)
                && note.expires_at.map(|ts| ts > now).unwrap_or(true)
        });

        if notes.is_empty() {
            return Ok(SearchResponse { items: Vec::new() });
        }

        let docs: Vec<String> = notes.iter().map(|note| note.text.clone()).collect();
        let scores = self
            .providers
            .rerank
            .rerank(&self.cfg.providers.rerank, &req.query, &docs)
            .await?;
        if scores.len() != notes.len() {
            return Err(ServiceError::Provider {
                message: "Rerank provider returned mismatched score count.".to_string(),
            });
        }

        let mut scored = Vec::with_capacity(notes.len());
        for (note, rerank_score) in notes.into_iter().zip(scores.into_iter()) {
            let age_days = (now - note.updated_at).as_seconds_f32() / 86_400.0;
            let decay = if self.cfg.ranking.recency_tau_days > 0.0 {
                (-age_days / self.cfg.ranking.recency_tau_days).exp()
            } else {
                1.0
            };
            let base = (1.0 + 0.6 * note.importance) * decay;
            let final_score = rerank_score + self.cfg.ranking.tie_breaker_weight * base;
            scored.push((note, final_score));
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k as usize);

        if req.record_hits.unwrap_or(false) {
            record_hits(&self.db.pool, &req.query, &scored, now).await?;
        }

        let items = scored
            .into_iter()
            .map(|(note, final_score)| SearchItem {
                note_id: note.note_id,
                note_type: note.r#type,
                key: note.key,
                scope: note.scope,
                text: note.text,
                importance: note.importance,
                confidence: note.confidence,
                updated_at: note.updated_at,
                expires_at: note.expires_at,
                final_score,
                source_ref: note.source_ref,
            })
            .collect();

        Ok(SearchResponse { items })
    }
}

fn resolve_scopes(cfg: &elf_config::Config, profile: &str) -> ServiceResult<Vec<String>> {
    match profile {
        "private_only" => Ok(cfg.scopes.read_profiles.private_only.clone()),
        "private_plus_project" => Ok(cfg.scopes.read_profiles.private_plus_project.clone()),
        "all_scopes" => Ok(cfg.scopes.read_profiles.all_scopes.clone()),
        _ => Err(ServiceError::InvalidRequest {
            message: "Unknown read_profile.".to_string(),
        }),
    }
}

fn point_id_to_uuid(point_id: &qdrant_client::qdrant::PointId) -> Option<uuid::Uuid> {
    match &point_id.point_id_options {
        Some(PointIdOptions::Uuid(id)) => uuid::Uuid::parse_str(id).ok(),
        _ => None,
    }
}

async fn record_hits(
    pool: &sqlx::PgPool,
    query: &str,
    scored: &[(MemoryNote, f32)],
    now: time::OffsetDateTime,
) -> ServiceResult<()> {
    let query_hash = hash_query(query);
    let mut tx = pool.begin().await?;

    for (rank, (note, final_score)) in scored.iter().enumerate() {
        sqlx::query(
            "UPDATE memory_notes SET hit_count = hit_count + 1, last_hit_at = $1 WHERE note_id = $2",
        )
        .bind(now)
        .bind(note.note_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO memory_hits (hit_id, note_id, query_hash, rank, final_score, ts) VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(uuid::Uuid::new_v4())
        .bind(note.note_id)
        .bind(&query_hash)
        .bind(rank as i32)
        .bind(*final_score)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

fn hash_query(query: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(query, &mut hasher);
    format!("{:x}", hasher.finish())
}
