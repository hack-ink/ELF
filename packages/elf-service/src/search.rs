use std::{
	collections::{HashMap, HashSet},
	hash::Hasher,
};

use elf_domain::cjk::contains_cjk;
use elf_storage::{
	models::MemoryNote,
	qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME},
};
use qdrant_client::qdrant::{
	Condition, Document, Filter, Fusion, MinShould, PrefetchQueryBuilder, Query,
	QueryPointsBuilder, ScoredPoint, point_id::PointIdOptions,
};
use serde::de::DeserializeOwned;
use sqlx::Row;
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
pub struct SearchBoost {
	pub name: String,
	pub score: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchExplain {
	pub retrieval_score: Option<f32>,
	pub retrieval_rank: Option<u32>,
	pub rerank_score: f32,
	pub tie_breaker_score: f32,
	pub final_score: f32,
	pub boosts: Vec<SearchBoost>,
	pub matched_terms: Vec<String>,
	pub matched_fields: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchItem {
	pub result_handle: uuid::Uuid,
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
	pub explain: SearchExplain,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResponse {
	pub trace_id: uuid::Uuid,
	pub items: Vec<SearchItem>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchExplainRequest {
	pub result_handle: uuid::Uuid,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchTrace {
	pub trace_id: uuid::Uuid,
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub read_profile: String,
	pub query: String,
	pub expansion_mode: String,
	pub expanded_queries: Vec<String>,
	pub allowed_scopes: Vec<String>,
	pub candidate_count: u32,
	pub top_k: u32,
	pub config_snapshot: serde_json::Value,
	#[serde(with = "crate::time_serde")]
	pub created_at: time::OffsetDateTime,
	pub trace_version: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchExplainItem {
	pub result_handle: uuid::Uuid,
	pub note_id: uuid::Uuid,
	pub rank: u32,
	pub explain: SearchExplain,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchExplainResponse {
	pub trace: SearchTrace,
	pub item: SearchExplainItem,
}

const TRACE_VERSION: i32 = 1;
const MAX_MATCHED_TERMS: usize = 8;

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

#[derive(Debug, Clone, Copy)]
struct RetrievalInfo {
	score: f32,
	rank: u32,
}

#[derive(Debug, Clone)]
struct Candidate {
	note_id: uuid::Uuid,
	retrieval_score: f32,
	retrieval_rank: u32,
}

#[derive(Debug)]
struct ScoredNote {
	note: MemoryNote,
	rerank_score: f32,
	tie_breaker_score: f32,
	final_score: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TracePayload {
	trace: TraceRecord,
	items: Vec<TraceItemRecord>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TraceRecord {
	trace_id: uuid::Uuid,
	tenant_id: String,
	project_id: String,
	agent_id: String,
	read_profile: String,
	query: String,
	expansion_mode: String,
	expanded_queries: Vec<String>,
	allowed_scopes: Vec<String>,
	candidate_count: u32,
	top_k: u32,
	config_snapshot: serde_json::Value,
	trace_version: i32,
	created_at: time::OffsetDateTime,
	expires_at: time::OffsetDateTime,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TraceItemRecord {
	item_id: uuid::Uuid,
	note_id: uuid::Uuid,
	rank: u32,
	retrieval_score: Option<f32>,
	retrieval_rank: Option<u32>,
	rerank_score: f32,
	tie_breaker_score: f32,
	final_score: f32,
	boosts: Vec<SearchBoost>,
	matched_terms: Vec<String>,
	matched_fields: Vec<String>,
}

struct TraceContext<'a> {
	trace_id: uuid::Uuid,
	tenant_id: &'a str,
	project_id: &'a str,
	agent_id: &'a str,
	read_profile: &'a str,
	query: &'a str,
	expansion_mode: ExpansionMode,
	expanded_queries: Vec<String>,
	allowed_scopes: &'a [String],
	candidate_count: usize,
	top_k: u32,
}

struct SearchTraceBuilder {
	trace: TraceRecord,
	items: Vec<TraceItemRecord>,
}

impl SearchTraceBuilder {
	fn new(context: TraceContext<'_>, cfg: &elf_config::Config, now: time::OffsetDateTime) -> Self {
		let trace = TraceRecord {
			trace_id: context.trace_id,
			tenant_id: context.tenant_id.to_string(),
			project_id: context.project_id.to_string(),
			agent_id: context.agent_id.to_string(),
			read_profile: context.read_profile.to_string(),
			query: context.query.to_string(),
			expansion_mode: expansion_mode_label(context.expansion_mode).to_string(),
			expanded_queries: context.expanded_queries,
			allowed_scopes: context.allowed_scopes.to_vec(),
			candidate_count: context.candidate_count as u32,
			top_k: context.top_k,
			config_snapshot: build_config_snapshot(cfg),
			trace_version: TRACE_VERSION,
			created_at: now,
			expires_at: now + time::Duration::days(cfg.search.explain.retention_days),
		};
		Self { trace, items: Vec::new() }
	}

	fn push_item(&mut self, item: TraceItemRecord) {
		self.items.push(item);
	}

	fn build(self) -> TracePayload {
		TracePayload { trace: self.trace, items: self.items }
	}
}

struct FinishSearchArgs<'a> {
	trace_id: uuid::Uuid,
	query: &'a str,
	tenant_id: &'a str,
	project_id: &'a str,
	agent_id: &'a str,
	read_profile: &'a str,
	allowed_scopes: &'a [String],
	expanded_queries: Vec<String>,
	expansion_mode: ExpansionMode,
	candidates: Vec<Candidate>,
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

		let top_k = req.top_k.unwrap_or(self.cfg.memory.top_k).max(1);
		let candidate_k = req.candidate_k.unwrap_or(self.cfg.memory.candidate_k).max(top_k);
		let query = req.query.clone();
		let read_profile = req.read_profile.clone();
		let record_hits_enabled = req.record_hits.unwrap_or(false);
		let expansion_mode = resolve_expansion_mode(&self.cfg);
		let trace_id = uuid::Uuid::new_v4();

		let allowed_scopes = resolve_scopes(&self.cfg, &read_profile)?;
		if allowed_scopes.is_empty() {
			return self
				.finish_search(FinishSearchArgs {
					trace_id,
					query: &query,
					tenant_id,
					project_id,
					agent_id,
					read_profile: &read_profile,
					allowed_scopes: &allowed_scopes,
					expanded_queries: vec![query.clone()],
					expansion_mode,
					candidates: Vec::new(),
					top_k,
					record_hits_enabled,
				})
				.await;
		}

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
			let candidates = collect_candidates(
				&baseline_points,
				self.cfg.search.prefilter.max_candidates,
				candidate_k,
			);
			let should_expand =
				should_expand_dynamic(baseline_points.len(), top_score, &self.cfg.search.dynamic);
			if !should_expand {
				return self
					.finish_search(FinishSearchArgs {
						trace_id,
						query: &query,
						tenant_id,
						project_id,
						agent_id,
						read_profile: &read_profile,
						allowed_scopes: &allowed_scopes,
						expanded_queries: vec![query.clone()],
						expansion_mode,
						candidates,
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

		let expanded_queries = queries.clone();
		let query_embeddings =
			self.embed_queries(&queries, &query, baseline_vector.as_ref()).await?;
		let fusion_points = self.run_fusion_query(&query_embeddings, &filter, candidate_k).await?;
		let candidates = collect_candidates(
			&fusion_points,
			self.cfg.search.prefilter.max_candidates,
			candidate_k,
		);

		self.finish_search(FinishSearchArgs {
			trace_id,
			query: &query,
			tenant_id,
			project_id,
			agent_id,
			read_profile: &read_profile,
			allowed_scopes: &allowed_scopes,
			expanded_queries,
			expansion_mode,
			candidates,
			top_k,
			record_hits_enabled,
		})
		.await
	}

	pub async fn search_explain(
		&self,
		req: SearchExplainRequest,
	) -> ServiceResult<SearchExplainResponse> {
		let row = sqlx::query(
			"SELECT \
                t.trace_id, t.tenant_id, t.project_id, t.agent_id, t.read_profile, t.query, \
                t.expansion_mode, t.expanded_queries, t.allowed_scopes, t.candidate_count, \
                t.top_k, t.config_snapshot, t.trace_version, t.created_at, \
                i.item_id, i.note_id, i.rank, i.retrieval_score, i.retrieval_rank, \
                i.rerank_score, i.tie_breaker_score, i.final_score, i.boosts, \
                i.matched_terms, i.matched_fields \
             FROM search_trace_items i \
             JOIN search_traces t ON i.trace_id = t.trace_id \
             WHERE i.item_id = $1",
		)
		.bind(req.result_handle)
		.fetch_optional(&self.db.pool)
		.await?;

		let Some(row) = row else {
			return Err(ServiceError::InvalidRequest {
				message: "Unknown result_handle or trace not yet persisted.".to_string(),
			});
		};

		let expanded_queries: Vec<String> =
			decode_json(row.try_get("expanded_queries")?, "expanded_queries")?;
		let allowed_scopes: Vec<String> =
			decode_json(row.try_get("allowed_scopes")?, "allowed_scopes")?;
		let config_snapshot: serde_json::Value = row.try_get("config_snapshot")?;
		let boosts: Vec<SearchBoost> = decode_json(row.try_get("boosts")?, "boosts")?;
		let matched_terms: Vec<String> =
			decode_json(row.try_get("matched_terms")?, "matched_terms")?;
		let matched_fields: Vec<String> =
			decode_json(row.try_get("matched_fields")?, "matched_fields")?;

		let trace = SearchTrace {
			trace_id: row.try_get("trace_id")?,
			tenant_id: row.try_get("tenant_id")?,
			project_id: row.try_get("project_id")?,
			agent_id: row.try_get("agent_id")?,
			read_profile: row.try_get("read_profile")?,
			query: row.try_get("query")?,
			expansion_mode: row.try_get("expansion_mode")?,
			expanded_queries,
			allowed_scopes,
			candidate_count: row.try_get::<i32, _>("candidate_count")? as u32,
			top_k: row.try_get::<i32, _>("top_k")? as u32,
			config_snapshot,
			created_at: row.try_get("created_at")?,
			trace_version: row.try_get("trace_version")?,
		};

		let explain = SearchExplain {
			retrieval_score: row.try_get("retrieval_score")?,
			retrieval_rank: row
				.try_get::<Option<i32>, _>("retrieval_rank")?
				.map(|rank| rank as u32),
			rerank_score: row.try_get("rerank_score")?,
			tie_breaker_score: row.try_get("tie_breaker_score")?,
			final_score: row.try_get("final_score")?,
			boosts,
			matched_terms,
			matched_fields,
		};

		let item = SearchExplainItem {
			result_handle: row.try_get("item_id")?,
			note_id: row.try_get("note_id")?,
			rank: row.try_get::<i32, _>("rank")? as u32,
			explain,
		};

		Ok(SearchExplainResponse { trace, item })
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
			trace_id,
			query,
			tenant_id,
			project_id,
			agent_id,
			read_profile,
			allowed_scopes,
			expanded_queries,
			expansion_mode,
			candidates,
			top_k,
			record_hits_enabled,
		} = args;
		let now = time::OffsetDateTime::now_utc();
		let candidate_ids: Vec<uuid::Uuid> =
			candidates.iter().map(|candidate| candidate.note_id).collect();
		let retrieval_map: HashMap<uuid::Uuid, RetrievalInfo> = candidates
			.iter()
			.map(|candidate| {
				(
					candidate.note_id,
					RetrievalInfo {
						score: candidate.retrieval_score,
						rank: candidate.retrieval_rank,
					},
				)
			})
			.collect();

		let mut notes: Vec<MemoryNote> = if candidate_ids.is_empty() {
			Vec::new()
		} else {
			sqlx::query_as(
				"SELECT * FROM memory_notes WHERE note_id = ANY($1) AND tenant_id = $2 AND project_id = $3",
			)
			.bind(&candidate_ids)
			.bind(tenant_id)
			.bind(project_id)
			.fetch_all(&self.db.pool)
			.await?
		};

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

		let mut scored: Vec<ScoredNote> = Vec::new();
		if !notes.is_empty() {
			let docs: Vec<String> = notes.iter().map(|note| note.text.clone()).collect();
			let scores =
				self.providers.rerank.rerank(&self.cfg.providers.rerank, query, &docs).await?;
			if scores.len() != notes.len() {
				return Err(ServiceError::Provider {
					message: "Rerank provider returned mismatched score count.".to_string(),
				});
			}

			scored = Vec::with_capacity(notes.len());
			for (note, rerank_score) in notes.into_iter().zip(scores.into_iter()) {
				let age_days = (now - note.updated_at).as_seconds_f32() / 86_400.0;
				let decay = if self.cfg.ranking.recency_tau_days > 0.0 {
					(-age_days / self.cfg.ranking.recency_tau_days).exp()
				} else {
					1.0
				};
				let base = (1.0 + 0.6 * note.importance) * decay;
				let tie_breaker_score = self.cfg.ranking.tie_breaker_weight * base;
				let final_score = rerank_score + tie_breaker_score;
				scored.push(ScoredNote { note, rerank_score, tie_breaker_score, final_score });
			}

			scored.sort_by(|a, b| {
				b.final_score.partial_cmp(&a.final_score).unwrap_or(std::cmp::Ordering::Equal)
			});
			scored.truncate(top_k as usize);
		}

		if record_hits_enabled && !scored.is_empty() {
			record_hits(&self.db.pool, query, &scored, now).await?;
		}

		let query_tokens = tokenize_query(query, MAX_MATCHED_TERMS);
		let mut items = Vec::with_capacity(scored.len());
		let trace_context = TraceContext {
			trace_id,
			tenant_id,
			project_id,
			agent_id,
			read_profile,
			query,
			expansion_mode,
			expanded_queries,
			allowed_scopes,
			candidate_count: candidates.len(),
			top_k,
		};
		let mut trace_builder = SearchTraceBuilder::new(trace_context, &self.cfg, now);
		for (idx, scored_note) in scored.into_iter().enumerate() {
			let rank = idx as u32 + 1;
			let retrieval = retrieval_map.get(&scored_note.note.note_id).copied();
			let (matched_terms, matched_fields) =
				match_terms(&query_tokens, &scored_note.note, MAX_MATCHED_TERMS);
			let boosts = vec![SearchBoost {
				name: "recency_importance".to_string(),
				score: scored_note.tie_breaker_score,
			}];
			let explain = SearchExplain {
				retrieval_score: retrieval.map(|entry| entry.score),
				retrieval_rank: retrieval.map(|entry| entry.rank),
				rerank_score: scored_note.rerank_score,
				tie_breaker_score: scored_note.tie_breaker_score,
				final_score: scored_note.final_score,
				boosts: boosts.clone(),
				matched_terms: matched_terms.clone(),
				matched_fields: matched_fields.clone(),
			};
			let result_handle = uuid::Uuid::new_v4();
			let note = scored_note.note;
			items.push(SearchItem {
				result_handle,
				note_id: note.note_id,
				note_type: note.r#type,
				key: note.key,
				scope: note.scope,
				text: note.text,
				importance: note.importance,
				confidence: note.confidence,
				updated_at: note.updated_at,
				expires_at: note.expires_at,
				final_score: scored_note.final_score,
				source_ref: note.source_ref,
				explain,
			});
			trace_builder.push_item(TraceItemRecord {
				item_id: result_handle,
				note_id: note.note_id,
				rank,
				retrieval_score: retrieval.map(|entry| entry.score),
				retrieval_rank: retrieval.map(|entry| entry.rank),
				rerank_score: scored_note.rerank_score,
				tie_breaker_score: scored_note.tie_breaker_score,
				final_score: scored_note.final_score,
				boosts,
				matched_terms,
				matched_fields,
			});
		}

		let trace_payload = trace_builder.build();
		if let Err(err) = enqueue_trace(&self.db.pool, trace_payload).await {
			tracing::error!(error = %err, trace_id = %trace_id, "Failed to enqueue search trace.");
		}

		Ok(SearchResponse { trace_id, items })
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

fn collect_candidates(
	points: &[ScoredPoint],
	max_candidates: u32,
	candidate_k: u32,
) -> Vec<Candidate> {
	let limit = if max_candidates == 0 || max_candidates >= candidate_k {
		points.len()
	} else {
		max_candidates as usize
	};
	let mut out = Vec::new();
	let mut seen = HashSet::new();
	for (idx, point) in points.iter().take(limit).enumerate() {
		let Some(id) = point.id.as_ref().and_then(point_id_to_uuid) else {
			continue;
		};
		if seen.insert(id) {
			out.push(Candidate {
				note_id: id,
				retrieval_score: point.score,
				retrieval_rank: idx as u32 + 1,
			});
		}
	}
	out
}

fn expansion_mode_label(mode: ExpansionMode) -> &'static str {
	match mode {
		ExpansionMode::Off => "off",
		ExpansionMode::Always => "always",
		ExpansionMode::Dynamic => "dynamic",
	}
}

fn tokenize_query(query: &str, max_terms: usize) -> Vec<String> {
	let mut normalized = String::with_capacity(query.len());
	for ch in query.chars() {
		if ch.is_ascii_alphanumeric() {
			normalized.push(ch.to_ascii_lowercase());
		} else {
			normalized.push(' ');
		}
	}

	let mut out = Vec::new();
	let mut seen = HashSet::new();
	for token in normalized.split_whitespace() {
		if token.len() < 2 {
			continue;
		}
		if seen.insert(token) {
			out.push(token.to_string());
		}
		if out.len() >= max_terms {
			break;
		}
	}
	out
}

fn match_terms(
	tokens: &[String],
	note: &MemoryNote,
	max_terms: usize,
) -> (Vec<String>, Vec<String>) {
	if tokens.is_empty() {
		return (Vec::new(), Vec::new());
	}
	let text = note.text.to_lowercase();
	let key = note.key.as_ref().map(|value| value.to_lowercase());
	let mut matched_terms = Vec::new();
	let mut matched_fields = HashSet::new();
	for token in tokens {
		let mut matched = false;
		if text.contains(token) {
			matched_fields.insert("text");
			matched = true;
		}
		if let Some(key) = key.as_ref()
			&& key.contains(token)
		{
			matched_fields.insert("key");
			matched = true;
		}
		if matched {
			matched_terms.push(token.clone());
		}
		if matched_terms.len() >= max_terms {
			break;
		}
	}
	let mut fields: Vec<String> =
		matched_fields.into_iter().map(|field| field.to_string()).collect();
	fields.sort();
	(matched_terms, fields)
}

fn decode_json<T: DeserializeOwned>(value: serde_json::Value, label: &str) -> ServiceResult<T> {
	serde_json::from_value(value)
		.map_err(|err| ServiceError::Storage { message: format!("Invalid {label} value: {err}") })
}

fn build_config_snapshot(cfg: &elf_config::Config) -> serde_json::Value {
	serde_json::json!({
		"search": {
			"expansion": {
				"mode": cfg.search.expansion.mode.as_str(),
				"max_queries": cfg.search.expansion.max_queries,
				"include_original": cfg.search.expansion.include_original,
			},
			"dynamic": {
				"min_candidates": cfg.search.dynamic.min_candidates,
				"min_top_score": cfg.search.dynamic.min_top_score,
			},
			"prefilter": {
				"max_candidates": cfg.search.prefilter.max_candidates,
			},
			"explain": {
				"retention_days": cfg.search.explain.retention_days,
			},
		},
		"ranking": {
			"recency_tau_days": cfg.ranking.recency_tau_days,
			"tie_breaker_weight": cfg.ranking.tie_breaker_weight,
		},
		"providers": {
			"embedding": {
				"provider_id": cfg.providers.embedding.provider_id.as_str(),
				"model": cfg.providers.embedding.model.as_str(),
				"dimensions": cfg.providers.embedding.dimensions,
			},
			"rerank": {
				"provider_id": cfg.providers.rerank.provider_id.as_str(),
				"model": cfg.providers.rerank.model.as_str(),
			},
		},
		"storage": {
			"qdrant": {
				"vector_dim": cfg.storage.qdrant.vector_dim,
				"collection": cfg.storage.qdrant.collection.as_str(),
			},
		},
	})
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

async fn enqueue_trace(pool: &sqlx::PgPool, payload: TracePayload) -> ServiceResult<()> {
	let now = time::OffsetDateTime::now_utc();
	let payload_json = serde_json::to_value(&payload).map_err(|err| ServiceError::Storage {
		message: format!("Failed to encode search trace payload: {err}"),
	})?;
	sqlx::query(
        "INSERT INTO search_trace_outbox \
         (outbox_id, trace_id, status, attempts, last_error, available_at, payload, created_at, updated_at) \
         VALUES ($1,$2,'PENDING',0,NULL,$3,$4,$3,$3)",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(payload.trace.trace_id)
    .bind(now)
    .bind(payload_json)
    .execute(pool)
    .await?;
	Ok(())
}

async fn record_hits(
	pool: &sqlx::PgPool,
	query: &str,
	scored: &[ScoredNote],
	now: time::OffsetDateTime,
) -> ServiceResult<()> {
	let query_hash = hash_query(query);
	let mut tx = pool.begin().await?;

	for (rank, scored_note) in scored.iter().enumerate() {
		let note = &scored_note.note;
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
        .bind(scored_note.final_score)
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

fn hash_cache_key(payload: &serde_json::Value) -> ServiceResult<String> {
	let raw = serde_json::to_vec(payload).map_err(|err| ServiceError::Storage {
		message: format!("Failed to encode cache key payload: {err}"),
	})?;
	Ok(blake3::hash(&raw).to_hex().to_string())
}

fn build_expansion_cache_key(
	query: &str,
	version: &str,
	max_queries: u32,
	include_original: bool,
	provider_id: &str,
	model: &str,
	temperature: f32,
) -> ServiceResult<String> {
	let payload = serde_json::json!({
		"kind": "expansion",
		"query": query.trim(),
		"provider_id": provider_id,
		"model": model,
		"temperature": temperature,
		"version": version,
		"max_queries": max_queries,
		"include_original": include_original,
	});
	hash_cache_key(&payload)
}

fn build_rerank_cache_key(
	query: &str,
	version: &str,
	provider_id: &str,
	model: &str,
	candidates: &[(uuid::Uuid, time::OffsetDateTime)],
) -> ServiceResult<String> {
	let signature: Vec<serde_json::Value> = candidates
		.iter()
		.map(|(note_id, updated_at)| {
			serde_json::json!({
				"note_id": note_id,
				"updated_at": updated_at,
			})
		})
		.collect();
	let payload = serde_json::json!({
		"kind": "rerank",
		"query": query.trim(),
		"provider_id": provider_id,
		"model": model,
		"version": version,
		"candidates": signature,
	});
	hash_cache_key(&payload)
}

#[cfg(test)]
mod tests {
	use super::{
		build_expansion_cache_key, build_rerank_cache_key, normalize_queries, should_expand_dynamic,
	};

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

	#[test]
	fn expansion_cache_key_changes_with_version() {
		let key_a =
			build_expansion_cache_key("alpha", "v1", 4, true, "llm", "model", 0.1_f32)
				.expect("Expected cache key.");
		let key_b =
			build_expansion_cache_key("alpha", "v2", 4, true, "llm", "model", 0.1_f32)
				.expect("Expected cache key.");
		assert_ne!(key_a, key_b);
	}

	#[test]
	fn rerank_cache_key_changes_with_updated_at() {
		let ts_a = time::OffsetDateTime::from_unix_timestamp(1).expect("Valid timestamp.");
		let ts_b = time::OffsetDateTime::from_unix_timestamp(2).expect("Valid timestamp.");
		let note_id = uuid::Uuid::new_v4();
		let key_a = build_rerank_cache_key(
			"q",
			"v1",
			"rerank",
			"model",
			&vec![(note_id, ts_a)],
		)
		.expect("Expected cache key.");
		let key_b = build_rerank_cache_key(
			"q",
			"v1",
			"rerank",
			"model",
			&vec![(note_id, ts_b)],
		)
		.expect("Expected cache key.");
		assert_ne!(key_a, key_b);
	}
}
