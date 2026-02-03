use std::{collections::HashSet, hash::Hasher};

use elf_domain::cjk::contains_cjk;
use elf_storage::{
	models::MemoryNote,
	qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME},
};
use qdrant_client::qdrant::{
	Condition, Document, Filter, Fusion, MinShould, PrefetchQueryBuilder, Query,
	QueryPointsBuilder, ScoredPoint, point_id::PointIdOptions,
};
use tracing::warn;

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
	#[serde(with = "crate::time_serde")]
	pub updated_at: time::OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	pub expires_at: Option<time::OffsetDateTime>,
	pub final_score: f32,
	pub source_ref: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResponse {
	pub items: Vec<SearchItem>,
}

#[derive(Debug, Clone)]
struct QueryEmbedding {
	text: String,
	vector: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpansionMode {
	Off,
	Always,
	Dynamic,
}

struct FinishSearchArgs<'a> {
	query: &'a str,
	tenant_id: &'a str,
	project_id: &'a str,
	agent_id: &'a str,
	allowed_scopes: &'a [String],
	candidate_ids: Vec<uuid::Uuid>,
	top_k: u32,
	record_hits_enabled: bool,
}

impl ElfService {
	pub async fn search(&self, req: SearchRequest) -> ServiceResult<SearchResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();
		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(ServiceError::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}
		if contains_cjk(&req.query) {
			return Err(ServiceError::NonEnglishInput { field: "$.query".to_string() });
		}

		let allowed_scopes = resolve_scopes(&self.cfg, &req.read_profile)?;
		if allowed_scopes.is_empty() {
			return Ok(SearchResponse { items: Vec::new() });
		}

		let top_k = req.top_k.unwrap_or(self.cfg.memory.top_k).max(1);
		let candidate_k = req.candidate_k.unwrap_or(self.cfg.memory.candidate_k).max(top_k);
		let query = req.query.clone();
		let record_hits_enabled = req.record_hits.unwrap_or(false);
		let expansion_mode = resolve_expansion_mode(&self.cfg);

		let private_scope = "agent_private".to_string();
		let non_private_scopes: Vec<String> =
			allowed_scopes.iter().filter(|scope| *scope != "agent_private").cloned().collect();
		let mut should_conditions = Vec::new();
		if allowed_scopes.iter().any(|scope| scope == "agent_private") {
			let private_filter = Filter::all([
				Condition::matches("scope", private_scope),
				Condition::matches("agent_id", agent_id.to_string()),
			]);
			should_conditions.push(Condition::from(private_filter));
		}
		if !non_private_scopes.is_empty() {
			should_conditions.push(Condition::matches("scope", non_private_scopes));
		}

		let (should, min_should) = if should_conditions.is_empty() {
			(Vec::new(), None)
		} else {
			(Vec::new(), Some(MinShould { min_count: 1, conditions: should_conditions }))
		};

		let filter = Filter {
			must: vec![
				Condition::matches("tenant_id", tenant_id.to_string()),
				Condition::matches("project_id", project_id.to_string()),
				Condition::matches("status", "active".to_string()),
			],
			should,
			must_not: Vec::new(),
			min_should,
		};

		let mut baseline_vector: Option<Vec<f32>> = None;
		if expansion_mode == ExpansionMode::Dynamic {
			let query_vec = self.embed_single_query(&query).await?;
			baseline_vector = Some(query_vec.clone());
			let baseline_points = self
				.run_fusion_query(
					&[QueryEmbedding { text: query.clone(), vector: query_vec }],
					&filter,
					candidate_k,
				)
				.await?;
			let top_score = baseline_points.first().map(|point| point.score).unwrap_or(0.0);
			let candidate_ids = collect_candidate_ids(
				&baseline_points,
				self.cfg.search.prefilter.max_candidates,
				candidate_k,
			);
			let should_expand =
				should_expand_dynamic(baseline_points.len(), top_score, &self.cfg.search.dynamic);
			if !should_expand {
				return self
					.finish_search(FinishSearchArgs {
						query: &query,
						tenant_id,
						project_id,
						agent_id,
						allowed_scopes: &allowed_scopes,
						candidate_ids,
						top_k,
						record_hits_enabled,
					})
					.await;
			}
		}

		let queries = match expansion_mode {
			ExpansionMode::Off => vec![query.clone()],
			ExpansionMode::Always | ExpansionMode::Dynamic => self.expand_queries(&query).await,
		};

		let query_embeddings =
			self.embed_queries(&queries, &query, baseline_vector.as_ref()).await?;
		let fusion_points = self.run_fusion_query(&query_embeddings, &filter, candidate_k).await?;
		let candidate_ids = collect_candidate_ids(
			&fusion_points,
			self.cfg.search.prefilter.max_candidates,
			candidate_k,
		);

		self.finish_search(FinishSearchArgs {
			query: &query,
			tenant_id,
			project_id,
			agent_id,
			allowed_scopes: &allowed_scopes,
			candidate_ids,
			top_k,
			record_hits_enabled,
		})
		.await
	}

	async fn embed_single_query(&self, query: &str) -> ServiceResult<Vec<f32>> {
		let embeddings = self
			.providers
			.embedding
			.embed(&self.cfg.providers.embedding, std::slice::from_ref(&query.to_string()))
			.await?;
		let query_vec = embeddings.into_iter().next().ok_or_else(|| ServiceError::Provider {
			message: "Embedding provider returned no vectors.".to_string(),
		})?;
		if query_vec.len() != self.cfg.storage.qdrant.vector_dim as usize {
			return Err(ServiceError::Provider {
				message: "Embedding vector dimension mismatch.".to_string(),
			});
		}
		Ok(query_vec)
	}

	async fn embed_queries(
		&self,
		queries: &[String],
		original_query: &str,
		baseline_vector: Option<&Vec<f32>>,
	) -> ServiceResult<Vec<QueryEmbedding>> {
		let mut extra_queries = Vec::new();
		for query in queries {
			if baseline_vector.is_some() && query == original_query {
				continue;
			}
			extra_queries.push(query.clone());
		}

		let mut embedded_iter = if extra_queries.is_empty() {
			Vec::new().into_iter()
		} else {
			let embedded = self
				.providers
				.embedding
				.embed(&self.cfg.providers.embedding, &extra_queries)
				.await?;
			if embedded.len() != extra_queries.len() {
				return Err(ServiceError::Provider {
					message: "Embedding provider returned mismatched vector count.".to_string(),
				});
			}
			embedded.into_iter()
		};
		let mut out = Vec::with_capacity(queries.len());
		for query in queries {
			let vector = if baseline_vector.is_some() && query == original_query {
				baseline_vector
					.ok_or_else(|| ServiceError::Provider {
						message: "Embedding baseline vector is missing.".to_string(),
					})?
					.clone()
			} else {
				embedded_iter.next().ok_or_else(|| ServiceError::Provider {
					message: "Embedding provider returned no vectors.".to_string(),
				})?
			};
			if vector.len() != self.cfg.storage.qdrant.vector_dim as usize {
				return Err(ServiceError::Provider {
					message: "Embedding vector dimension mismatch.".to_string(),
				});
			}
			out.push(QueryEmbedding { text: query.clone(), vector });
		}
		Ok(out)
	}

	async fn run_fusion_query(
		&self,
		queries: &[QueryEmbedding],
		filter: &Filter,
		candidate_k: u32,
	) -> ServiceResult<Vec<ScoredPoint>> {
		let mut search = QueryPointsBuilder::new(self.qdrant.collection.clone());
		for query in queries {
			let dense_prefetch = PrefetchQueryBuilder::default()
				.query(Query::new_nearest(query.vector.clone()))
				.using(DENSE_VECTOR_NAME)
				.filter(filter.clone())
				.limit(candidate_k as u64);
			let bm25_prefetch = PrefetchQueryBuilder::default()
				.query(Query::new_nearest(Document::new(query.text.clone(), BM25_MODEL)))
				.using(BM25_VECTOR_NAME)
				.filter(filter.clone())
				.limit(candidate_k as u64);
			search = search.add_prefetch(dense_prefetch).add_prefetch(bm25_prefetch);
		}

		let search = search.query(Fusion::Rrf).limit(candidate_k as u64);
		let response = self
			.qdrant
			.client
			.query(search)
			.await
			.map_err(|err| ServiceError::Qdrant { message: err.to_string() })?;
		Ok(response.result)
	}

	async fn expand_queries(&self, query: &str) -> Vec<String> {
		let cfg = &self.cfg.search.expansion;
		let messages = build_expansion_messages(query, cfg.max_queries, cfg.include_original);
		let raw = match self
			.providers
			.extractor
			.extract(&self.cfg.providers.llm_extractor, &messages)
			.await
		{
			Ok(value) => value,
			Err(err) => {
				warn!(error = %err, "Query expansion failed; falling back to original query.");
				return vec![query.to_string()];
			},
		};

		let parsed: ExpansionOutput = match serde_json::from_value(raw) {
			Ok(value) => value,
			Err(err) => {
				warn!(error = %err, "Query expansion returned invalid JSON; falling back to original query.");
				return vec![query.to_string()];
			},
		};

		let normalized =
			normalize_queries(parsed.queries, query, cfg.include_original, cfg.max_queries);
		if normalized.is_empty() { vec![query.to_string()] } else { normalized }
	}

	async fn finish_search(&self, args: FinishSearchArgs<'_>) -> ServiceResult<SearchResponse> {
		let FinishSearchArgs {
			query,
			tenant_id,
			project_id,
			agent_id,
			allowed_scopes,
			candidate_ids,
			top_k,
			record_hits_enabled,
		} = args;
		if candidate_ids.is_empty() {
			return Ok(SearchResponse { items: Vec::new() });
		}

		let mut notes: Vec<MemoryNote> = sqlx::query_as(
			"SELECT * FROM memory_notes WHERE note_id = ANY($1) AND tenant_id = $2 AND project_id = $3",
		)
		.bind(&candidate_ids)
		.bind(tenant_id)
		.bind(project_id)
		.fetch_all(&self.db.pool)
		.await?;

		let now = time::OffsetDateTime::now_utc();
		notes.retain(|note| {
			if note.tenant_id != tenant_id || note.project_id != project_id {
				return false;
			}
			if note.scope == "agent_private" && note.agent_id != agent_id {
				return false;
			}
			note.status == "active"
				&& allowed_scopes.contains(&note.scope)
				&& note.expires_at.map(|ts| ts > now).unwrap_or(true)
		});

		if notes.is_empty() {
			return Ok(SearchResponse { items: Vec::new() });
		}

		let docs: Vec<String> = notes.iter().map(|note| note.text.clone()).collect();
		let scores = self.providers.rerank.rerank(&self.cfg.providers.rerank, query, &docs).await?;
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

		if record_hits_enabled {
			record_hits(&self.db.pool, query, &scored, now).await?;
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

#[derive(Debug, serde::Deserialize)]
struct ExpansionOutput {
	queries: Vec<String>,
}

fn resolve_expansion_mode(cfg: &elf_config::Config) -> ExpansionMode {
	match cfg.search.expansion.mode.as_str() {
		"off" => ExpansionMode::Off,
		"always" => ExpansionMode::Always,
		"dynamic" => ExpansionMode::Dynamic,
		_ => ExpansionMode::Off,
	}
}

fn should_expand_dynamic(
	candidate_count: usize,
	top_score: f32,
	cfg: &elf_config::SearchDynamic,
) -> bool {
	candidate_count < cfg.min_candidates as usize || top_score < cfg.min_top_score
}

fn normalize_queries(
	queries: Vec<String>,
	original: &str,
	include_original: bool,
	max_queries: u32,
) -> Vec<String> {
	let mut out = Vec::new();
	let mut seen = HashSet::new();

	if include_original {
		push_query(&mut out, &mut seen, original);
	}
	for query in queries {
		if out.len() >= max_queries as usize {
			break;
		}
		push_query(&mut out, &mut seen, &query);
	}
	out.truncate(max_queries as usize);
	out
}

fn push_query(out: &mut Vec<String>, seen: &mut HashSet<String>, value: &str) {
	let trimmed = value.trim();
	if trimmed.is_empty() || contains_cjk(trimmed) {
		return;
	}
	let key = trimmed.to_lowercase();
	if seen.insert(key) {
		out.push(trimmed.to_string());
	}
}

fn build_expansion_messages(
	query: &str,
	max_queries: u32,
	include_original: bool,
) -> Vec<serde_json::Value> {
	let schema = serde_json::json!({
		"queries": ["string"]
	});
	let schema_text = serde_json::to_string_pretty(&schema)
		.unwrap_or_else(|_| "{\"queries\": [\"string\"]}".to_string());
	let system_prompt = "You are a query expansion engine for a memory retrieval system. \
Output must be valid JSON only and must match the provided schema exactly. \
Generate short English-only query variations that preserve the original intent. \
Do not include any CJK characters. Do not add explanations or extra fields.";
	let user_prompt = format!(
		"Return JSON matching this exact schema:\n{schema}\nConstraints:\n- MAX_QUERIES = {max}\n- INCLUDE_ORIGINAL = {include}\nOriginal query:\n{query}",
		schema = schema_text,
		max = max_queries,
		include = include_original,
		query = query
	);
	vec![
		serde_json::json!({ "role": "system", "content": system_prompt }),
		serde_json::json!({ "role": "user", "content": user_prompt }),
	]
}

fn collect_candidate_ids(
	points: &[ScoredPoint],
	max_candidates: u32,
	candidate_k: u32,
) -> Vec<uuid::Uuid> {
	let limit = if max_candidates == 0 || max_candidates >= candidate_k {
		points.len()
	} else {
		max_candidates as usize
	};
	let mut out = Vec::new();
	let mut seen = HashSet::new();
	for point in points.iter().take(limit) {
		let Some(id) = point.id.as_ref().and_then(point_id_to_uuid) else {
			continue;
		};
		if seen.insert(id) {
			out.push(id);
		}
	}
	out
}

fn resolve_scopes(cfg: &elf_config::Config, profile: &str) -> ServiceResult<Vec<String>> {
	match profile {
		"private_only" => Ok(cfg.scopes.read_profiles.private_only.clone()),
		"private_plus_project" => Ok(cfg.scopes.read_profiles.private_plus_project.clone()),
		"all_scopes" => Ok(cfg.scopes.read_profiles.all_scopes.clone()),
		_ => Err(ServiceError::InvalidRequest { message: "Unknown read_profile.".to_string() }),
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

#[cfg(test)]
mod tests {
	use super::{normalize_queries, should_expand_dynamic};

	#[test]
	fn normalize_queries_includes_original_and_dedupes() {
		let queries = vec!["alpha".to_string(), "beta".to_string(), "alpha".to_string()];
		let normalized = normalize_queries(queries, "alpha", true, 4);
		assert_eq!(normalized, vec!["alpha".to_string(), "beta".to_string()]);
	}

	#[test]
	fn normalize_queries_respects_max_queries() {
		let queries =
			vec!["one".to_string(), "two".to_string(), "three".to_string(), "four".to_string()];
		let normalized = normalize_queries(queries, "zero", true, 3);
		assert_eq!(normalized.len(), 3);
	}

	#[test]
	fn dynamic_trigger_checks_candidates_and_score() {
		let cfg = elf_config::SearchDynamic { min_candidates: 10, min_top_score: 0.2 };
		assert!(should_expand_dynamic(5, 0.9, &cfg));
		assert!(should_expand_dynamic(20, 0.1, &cfg));
		assert!(!should_expand_dynamic(20, 0.9, &cfg));
	}
}
