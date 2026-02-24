use std::collections::{HashMap, HashSet};

use qdrant_client::qdrant::{
	Condition, Filter, Fusion, MinShould, PrefetchQueryBuilder, Query, QueryPointsBuilder,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, Result};
use elf_domain::cjk;
use elf_storage::{
	doc_outbox, docs as doc_store,
	models::{DocChunk, DocDocument},
	qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME},
};

use crate::access::{SharedSpaceGrantKey, load_shared_read_grants_with_org_shared};

const MAX_TOP_K: u32 = 32;
const MAX_CANDIDATE_K: u32 = 1_024;
const DEFAULT_DOC_MAX_BYTES: usize = 4 * 1024 * 1024;
const DEFAULT_CHUNK_TARGET_BYTES: usize = 2_048;
const DEFAULT_CHUNK_OVERLAP_BYTES: usize = 256;
const DEFAULT_MAX_CHUNKS_PER_DOC: usize = 4_096;
const DEFAULT_L1_MAX_BYTES: usize = 8 * 1024;
const DEFAULT_L2_MAX_BYTES: usize = 32 * 1024;

#[derive(Clone, Debug, Deserialize)]
pub struct DocsPutRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: String,
	pub title: Option<String>,
	#[serde(default)]
	pub source_ref: Value,
	pub content: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocsPutResponse {
	pub doc_id: Uuid,
	pub chunk_count: u32,
	pub content_bytes: u32,
	pub content_hash: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DocsGetRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub read_profile: String,
	pub doc_id: Uuid,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocsGetResponse {
	pub doc_id: Uuid,
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: String,
	pub status: String,
	pub title: Option<String>,
	pub source_ref: Value,
	pub content_bytes: u32,
	pub content_hash: String,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DocsSearchL0Request {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub read_profile: String,
	pub query: String,
	pub top_k: Option<u32>,
	pub candidate_k: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0Item {
	pub doc_id: Uuid,
	pub chunk_id: Uuid,
	pub score: f32,
	pub snippet: String,
	pub scope: String,
	pub project_id: String,
	pub agent_id: String,
	pub updated_at: OffsetDateTime,
	pub content_hash: String,
	pub chunk_hash: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0Response {
	pub items: Vec<DocsSearchL0Item>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TextQuoteSelector {
	pub exact: String,
	pub prefix: Option<String>,
	pub suffix: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TextPositionSelector {
	pub start: usize,
	pub end: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DocsExcerptsGetRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub read_profile: String,
	pub doc_id: Uuid,
	pub level: String, // "L1" | "L2"
	pub chunk_id: Option<Uuid>,
	pub quote: Option<TextQuoteSelector>,
	pub position: Option<TextPositionSelector>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocsExcerptVerification {
	pub verified: bool,
	pub verification_errors: Vec<String>,
	pub content_hash: String,
	pub excerpt_hash: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocsExcerptResponse {
	pub doc_id: Uuid,
	pub excerpt: String,
	pub start_offset: usize,
	pub end_offset: usize,
	pub verification: DocsExcerptVerification,
}

impl crate::ElfService {
	pub async fn docs_put(&self, req: DocsPutRequest) -> Result<DocsPutResponse> {
		validate_docs_put(&req)?;

		let now = OffsetDateTime::now_utc();
		let embed_version = crate::embedding_version(&self.cfg);

		let DocsPutRequest { tenant_id, project_id, agent_id, scope, title, source_ref, content } =
			req;

		let effective_project_id = if scope.trim() == "org_shared" {
			crate::access::ORG_PROJECT_ID
		} else {
			project_id.as_str()
		};

		let content_bytes = content.len();
		let content_hash = blake3::hash(content.as_bytes());
		let doc_id = Uuid::new_v4();
		let chunks = split_bytes_by_sentence(
			content.as_str(),
			DEFAULT_CHUNK_TARGET_BYTES,
			DEFAULT_CHUNK_OVERLAP_BYTES,
			DEFAULT_MAX_CHUNKS_PER_DOC,
		)?;

		let mut tx = self.db.pool.begin().await?;

		let doc_row = DocDocument {
			doc_id,
			tenant_id: tenant_id.clone(),
			project_id: effective_project_id.to_string(),
			agent_id: agent_id.clone(),
			scope: scope.clone(),
			status: "active".to_string(),
			title,
			source_ref: doc_store::normalize_source_ref(Some(source_ref)),
			content,
			content_bytes: content_bytes as i32,
			content_hash: content_hash.to_hex().to_string(),
			created_at: now,
			updated_at: now,
		};

		doc_store::insert_doc_document(&mut *tx, &doc_row).await?;

		for (chunk_index, chunk) in chunks.iter().enumerate() {
			let chunk_hash = blake3::hash(chunk.text.as_bytes());
			let chunk_row = DocChunk {
				chunk_id: chunk.chunk_id,
				doc_id,
				chunk_index: chunk_index as i32,
				start_offset: chunk.start_offset as i32,
				end_offset: chunk.end_offset as i32,
				chunk_text: chunk.text.clone(),
				chunk_hash: chunk_hash.to_hex().to_string(),
				created_at: now,
			};

			doc_store::insert_doc_chunk(&mut *tx, &chunk_row).await?;
			doc_outbox::enqueue_doc_outbox(
				&mut *tx,
				doc_id,
				chunk_row.chunk_id,
				"UPSERT",
				embed_version.as_str(),
			)
			.await?;
		}

		if scope.trim() != "agent_private" {
			crate::access::ensure_active_project_scope_grant(
				&mut *tx,
				tenant_id.as_str(),
				effective_project_id,
				scope.as_str(),
				agent_id.as_str(),
			)
			.await?;
		}

		tx.commit().await?;

		Ok(DocsPutResponse {
			doc_id,
			chunk_count: chunks.len() as u32,
			content_bytes: content_bytes as u32,
			content_hash: content_hash.to_hex().to_string(),
		})
	}

	pub async fn docs_get(&self, req: DocsGetRequest) -> Result<DocsGetResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();
		let read_profile = req.read_profile.trim();

		if tenant_id.is_empty()
			|| project_id.is_empty()
			|| agent_id.is_empty()
			|| read_profile.is_empty()
		{
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, agent_id, and read_profile are required."
					.to_string(),
			});
		}
		let allowed_scopes = crate::search::resolve_read_profile_scopes(&self.cfg, read_profile)?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");

		let row: Option<DocDocument> = sqlx::query_as::<_, DocDocument>(
			"\
SELECT
\tdoc_id,
\ttenant_id,
\tproject_id,
\tagent_id,
\tscope,
\tstatus,
\ttitle,
\tCOALESCE(source_ref, '{}'::jsonb) AS source_ref,
\tcontent,
\tcontent_bytes,
\tcontent_hash,
\tcreated_at,
\tupdated_at
FROM doc_documents
WHERE doc_id = $1
  AND tenant_id = $2
  AND (
    project_id = $3
    OR (project_id = $4 AND scope = 'org_shared')
  )
LIMIT 1",
		)
		.bind(req.doc_id)
		.bind(tenant_id)
		.bind(project_id)
		.bind(crate::access::ORG_PROJECT_ID)
		.fetch_optional(&self.db.pool)
		.await?;
		let Some(row) = row else {
			return Err(Error::NotFound { message: "Doc not found.".to_string() });
		};
		let shared_grants = if row.scope == "agent_private" {
			HashSet::new()
		} else {
			load_shared_read_grants_with_org_shared(
				&self.db.pool,
				tenant_id,
				project_id,
				agent_id,
				org_shared_allowed,
			)
			.await?
		};

		if row.status != "active"
			|| !doc_read_allowed(
				agent_id,
				&allowed_scopes,
				&shared_grants,
				row.agent_id.as_str(),
				row.scope.as_str(),
			) {
			return Err(Error::NotFound { message: "Doc not found.".to_string() });
		}

		Ok(DocsGetResponse {
			doc_id: row.doc_id,
			tenant_id: row.tenant_id,
			project_id: row.project_id,
			agent_id: row.agent_id,
			scope: row.scope,
			status: row.status,
			title: row.title,
			source_ref: row.source_ref,
			content_bytes: row.content_bytes.max(0) as u32,
			content_hash: row.content_hash,
			created_at: row.created_at,
			updated_at: row.updated_at,
		})
	}

	pub async fn docs_search_l0(&self, req: DocsSearchL0Request) -> Result<DocsSearchL0Response> {
		validate_docs_search_l0(&req)?;

		let top_k = req.top_k.unwrap_or(12).min(MAX_TOP_K);
		let candidate_k = req.candidate_k.unwrap_or(60).min(MAX_CANDIDATE_K);
		let allowed_scopes =
			crate::search::resolve_read_profile_scopes(&self.cfg, req.read_profile.as_str())?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let shared_grants = load_shared_read_grants_with_org_shared(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.agent_id.as_str(),
			org_shared_allowed,
		)
		.await?;

		let filter = build_doc_search_filter(
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.agent_id.as_str(),
			&allowed_scopes,
		);

		let embedded = self
			.providers
			.embedding
			.embed(&self.cfg.providers.embedding, &[req.query.clone()])
			.await?;
		let vector = embedded.first().ok_or_else(|| Error::Provider {
			message: "Embedding provider returned no vectors.".to_string(),
		})?;
		if vector.len() != self.cfg.storage.qdrant.vector_dim as usize {
			return Err(Error::Provider {
				message: "Embedding vector dimension mismatch.".to_string(),
			});
		}

		let scored = run_doc_fusion_query(
			&self.qdrant.client,
			self.cfg.storage.qdrant.docs_collection.as_str(),
			req.query.as_str(),
			vector,
			&filter,
			candidate_k,
		)
		.await?;

		let mut scored_chunks = Vec::new();
		let mut seen = HashSet::new();
		for point in scored.into_iter().take(candidate_k as usize) {
			let chunk_id = parse_scored_point_uuid_id(&point)?;
			if !seen.insert(chunk_id) {
				continue;
			}

			scored_chunks.push((chunk_id, point.score));
		}

		let chunk_ids: Vec<Uuid> = scored_chunks.iter().map(|(chunk_id, _)| *chunk_id).collect();
		let rows = load_doc_search_rows(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			&chunk_ids,
		)
		.await?;

		let mut items = Vec::with_capacity(top_k as usize);
		for (chunk_id, score) in scored_chunks {
			let Some(row) = rows.get(&chunk_id) else { continue };

			if !doc_read_allowed(
				req.agent_id.as_str(),
				&allowed_scopes,
				&shared_grants,
				row.agent_id.as_str(),
				row.scope.as_str(),
			) {
				continue;
			}

			items.push(DocsSearchL0Item {
				doc_id: row.doc_id,
				chunk_id,
				score,
				snippet: truncate_bytes(row.chunk_text.as_str(), 256),
				scope: row.scope.clone(),
				project_id: row.project_id.clone(),
				agent_id: row.agent_id.clone(),
				updated_at: row.updated_at,
				content_hash: row.content_hash.clone(),
				chunk_hash: row.chunk_hash.clone(),
			});
		}

		items.sort_by(|a, b| b.score.total_cmp(&a.score));
		items.truncate(top_k as usize);

		Ok(DocsSearchL0Response { items })
	}

	pub async fn docs_excerpts_get(
		&self,
		req: DocsExcerptsGetRequest,
	) -> Result<DocsExcerptResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();
		let read_profile = req.read_profile.trim();

		if tenant_id.is_empty()
			|| project_id.is_empty()
			|| agent_id.is_empty()
			|| read_profile.is_empty()
		{
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, agent_id, and read_profile are required."
					.to_string(),
			});
		}
		if let Some(quote) = req.quote.as_ref() {
			if cjk::contains_cjk(quote.exact.as_str()) {
				return Err(Error::NonEnglishInput { field: "$.quote.exact".to_string() });
			}
			if let Some(prefix) = quote.prefix.as_ref()
				&& cjk::contains_cjk(prefix.as_str())
			{
				return Err(Error::NonEnglishInput { field: "$.quote.prefix".to_string() });
			}
			if let Some(suffix) = quote.suffix.as_ref()
				&& cjk::contains_cjk(suffix.as_str())
			{
				return Err(Error::NonEnglishInput { field: "$.quote.suffix".to_string() });
			}
		}

		let allowed_scopes = crate::search::resolve_read_profile_scopes(&self.cfg, read_profile)?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let shared_grants = load_shared_read_grants_with_org_shared(
			&self.db.pool,
			tenant_id,
			project_id,
			agent_id,
			org_shared_allowed,
		)
		.await?;

		let row: Option<DocDocument> = sqlx::query_as::<_, DocDocument>(
			"\
SELECT
\tdoc_id,
\ttenant_id,
\tproject_id,
\tagent_id,
\tscope,
\tstatus,
\ttitle,
\tCOALESCE(source_ref, '{}'::jsonb) AS source_ref,
\tcontent,
\tcontent_bytes,
\tcontent_hash,
\tcreated_at,
\tupdated_at
FROM doc_documents
WHERE doc_id = $1
  AND tenant_id = $2
  AND (
    project_id = $3
    OR (project_id = $4 AND scope = 'org_shared')
  )
LIMIT 1",
		)
		.bind(req.doc_id)
		.bind(tenant_id)
		.bind(project_id)
		.bind(crate::access::ORG_PROJECT_ID)
		.fetch_optional(&self.db.pool)
		.await?;
		let Some(doc) = row else {
			return Err(Error::NotFound { message: "Doc not found.".to_string() });
		};

		if doc.status != "active"
			|| !doc_read_allowed(
				agent_id,
				&allowed_scopes,
				&shared_grants,
				doc.agent_id.as_str(),
				doc.scope.as_str(),
			) {
			return Err(Error::NotFound { message: "Doc not found.".to_string() });
		}

		let level_max = match req.level.as_str() {
			"L1" => DEFAULT_L1_MAX_BYTES,
			"L2" => DEFAULT_L2_MAX_BYTES,
			_ => {
				return Err(Error::InvalidRequest {
					message: "level must be L1 or L2.".to_string(),
				});
			},
		};

		let mut verification_errors = Vec::new();
		let mut verified = true;

		let (match_start, match_end) = if let Some(chunk_id) = req.chunk_id {
			let chunk = doc_store::get_doc_chunk(&self.db.pool, chunk_id).await?;
			let Some(chunk) = chunk else {
				return Err(Error::NotFound { message: "Chunk not found.".to_string() });
			};
			if chunk.doc_id != doc.doc_id {
				return Err(Error::NotFound { message: "Chunk not found.".to_string() });
			}

			(chunk.start_offset.max(0) as usize, chunk.end_offset.max(0) as usize)
		} else if let Some(quote) = req.quote.as_ref() {
			match locate_quote(&doc.content, quote) {
				Some((s, e)) => (s, e),
				None => {
					verified = false;
					verification_errors.push("QUOTE_SELECTOR_NOT_FOUND".to_string());

					if let Some(pos) = req.position.as_ref() {
						(pos.start.min(doc.content.len()), pos.end.min(doc.content.len()))
					} else {
						return Err(Error::NotFound {
							message: "Selector did not match document.".to_string(),
						});
					}
				},
			}
		} else if let Some(pos) = req.position.as_ref() {
			(pos.start.min(doc.content.len()), pos.end.min(doc.content.len()))
		} else {
			return Err(Error::InvalidRequest {
				message: "One of chunk_id, quote, or position is required.".to_string(),
			});
		};

		let (start, end) = bounded_window(match_start, match_end, doc.content.as_str(), level_max);
		let excerpt = doc.content.get(start..end).unwrap_or("").to_string();

		let excerpt_hash = blake3::hash(excerpt.as_bytes()).to_hex().to_string();
		let content_hash = doc.content_hash.clone();

		if excerpt.is_empty() {
			verified = false;
			verification_errors.push("EMPTY_EXCERPT".to_string());
		}

		Ok(DocsExcerptResponse {
			doc_id: doc.doc_id,
			excerpt,
			start_offset: start,
			end_offset: end,
			verification: DocsExcerptVerification {
				verified,
				verification_errors,
				content_hash,
				excerpt_hash,
			},
		})
	}
}

fn validate_docs_put(req: &DocsPutRequest) -> Result<()> {
	if req.content.trim().is_empty() {
		return Err(Error::InvalidRequest { message: "content must be non-empty.".to_string() });
	}
	if req.content.len() > DEFAULT_DOC_MAX_BYTES {
		return Err(Error::InvalidRequest {
			message: "content exceeds max_doc_bytes.".to_string(),
		});
	}
	if req.scope.trim().is_empty() {
		return Err(Error::InvalidRequest { message: "scope must be non-empty.".to_string() });
	}
	if !matches!(req.scope.as_str(), "agent_private" | "project_shared" | "org_shared") {
		return Err(Error::InvalidRequest { message: "Unknown scope.".to_string() });
	}
	if cjk::contains_cjk(req.content.as_str()) {
		return Err(Error::NonEnglishInput { field: "$.content".to_string() });
	}
	if let Some(title) = req.title.as_ref()
		&& cjk::contains_cjk(title.as_str())
	{
		return Err(Error::NonEnglishInput { field: "$.title".to_string() });
	}
	if let Some(found) = find_cjk_path(&req.source_ref, "$.source_ref") {
		return Err(Error::NonEnglishInput { field: found });
	}

	Ok(())
}

fn validate_docs_search_l0(req: &DocsSearchL0Request) -> Result<()> {
	if req.query.trim().is_empty() {
		return Err(Error::InvalidRequest { message: "query must be non-empty.".to_string() });
	}
	if cjk::contains_cjk(req.query.as_str()) {
		return Err(Error::NonEnglishInput { field: "$.query".to_string() });
	}

	Ok(())
}

fn find_cjk_path(value: &Value, path: &str) -> Option<String> {
	match value {
		Value::String(text) =>
			if cjk::contains_cjk(text) {
				Some(path.to_string())
			} else {
				None
			},
		Value::Array(items) => {
			for (idx, item) in items.iter().enumerate() {
				let child_path = format!("{path}[{idx}]");

				if let Some(found) = find_cjk_path(item, &child_path) {
					return Some(found);
				}
			}

			None
		},
		Value::Object(map) => {
			for (key, value) in map.iter() {
				let child_path = format!("{path}[\"{}\"]", escape_json_path_key(key));

				if let Some(found) = find_cjk_path(value, &child_path) {
					return Some(found);
				}
			}

			None
		},
		_ => None,
	}
}

fn escape_json_path_key(key: &str) -> String {
	key.replace('\\', "\\\\").replace('"', "\\\"")
}

#[derive(Clone, Debug)]
struct ByteChunk {
	chunk_id: Uuid,
	start_offset: usize,
	end_offset: usize,
	text: String,
}

fn split_bytes_by_sentence(
	text: &str,
	target_bytes: usize,
	overlap_bytes: usize,
	max_chunks: usize,
) -> Result<Vec<ByteChunk>> {
	use unicode_segmentation::UnicodeSegmentation;

	let sentences: Vec<(usize, &str)> = text.split_sentence_bound_indices().collect();
	let mut chunks = Vec::new();
	let mut current = String::new();
	let mut current_start = 0_usize;
	let mut last_end = 0_usize;

	for (idx, sentence) in sentences {
		let candidate = format!("{}{}", current, sentence);
		if candidate.len() > target_bytes && !current.is_empty() {
			chunks.push(ByteChunk {
				chunk_id: Uuid::new_v4(),
				start_offset: current_start,
				end_offset: last_end,
				text: current.clone(),
			});

			if chunks.len() >= max_chunks {
				return Err(Error::InvalidRequest {
					message: "doc exceeds max_chunks_per_doc.".to_string(),
				});
			}

			let overlap = overlap_tail_bytes(&current, overlap_bytes);
			current_start = last_end.saturating_sub(overlap.len());
			current = overlap;
		}
		if current.is_empty() {
			current_start = idx;
		}

		current.push_str(sentence);
		last_end = idx + sentence.len();
	}

	if !current.is_empty() {
		chunks.push(ByteChunk {
			chunk_id: Uuid::new_v4(),
			start_offset: current_start,
			end_offset: last_end,
			text: current,
		});
	}

	Ok(chunks)
}

fn overlap_tail_bytes(text: &str, overlap_bytes: usize) -> String {
	if overlap_bytes == 0 {
		return String::new();
	}
	let bytes = text.as_bytes();
	if bytes.len() <= overlap_bytes {
		return text.to_string();
	}
	let start = bytes.len().saturating_sub(overlap_bytes);
	let mut cut = start;
	while cut < bytes.len() && !text.is_char_boundary(cut) {
		cut += 1;
	}
	text.get(cut..).unwrap_or("").to_string()
}

async fn run_doc_fusion_query(
	client: &qdrant_client::Qdrant,
	collection: &str,
	query_text: &str,
	vector: &[f32],
	filter: &Filter,
	candidate_k: u32,
) -> Result<Vec<qdrant_client::qdrant::ScoredPoint>> {
	let mut search = QueryPointsBuilder::new(collection.to_string());

	let dense_prefetch = PrefetchQueryBuilder::default()
		.query(Query::new_nearest(vector.to_vec()))
		.using(DENSE_VECTOR_NAME)
		.filter(filter.clone())
		.limit(candidate_k as u64);
	let bm25_prefetch = PrefetchQueryBuilder::default()
		.query(Query::new_nearest(qdrant_client::qdrant::Document::new(
			query_text.to_string(),
			BM25_MODEL,
		)))
		.using(BM25_VECTOR_NAME)
		.filter(filter.clone())
		.limit(candidate_k as u64);

	search = search.add_prefetch(dense_prefetch).add_prefetch(bm25_prefetch);

	let search = search.with_payload(false).query(Fusion::Rrf).limit(candidate_k as u64);
	let response =
		client.query(search).await.map_err(|err| Error::Qdrant { message: err.to_string() })?;

	Ok(response.result)
}

fn build_doc_search_filter(
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	allowed_scopes: &[String],
) -> Filter {
	let private_scope = "agent_private".to_string();
	let non_private_scopes: Vec<String> =
		allowed_scopes.iter().filter(|scope| *scope != "agent_private").cloned().collect();
	let mut scope_should_conditions = Vec::new();

	if allowed_scopes.iter().any(|scope| scope == "agent_private") {
		let private_filter = Filter::all([
			Condition::matches("scope", private_scope),
			Condition::matches("agent_id", agent_id.to_string()),
		]);

		scope_should_conditions.push(Condition::from(private_filter));
	}
	if !non_private_scopes.is_empty() {
		scope_should_conditions.push(Condition::matches("scope", non_private_scopes));
	}

	let scope_min_should = if scope_should_conditions.is_empty() {
		None
	} else {
		Some(MinShould { min_count: 1, conditions: scope_should_conditions })
	};
	let mut project_or_org_branches = vec![Condition::from(Filter {
		must: vec![Condition::matches("project_id", project_id.to_string())],
		should: Vec::new(),
		must_not: Vec::new(),
		min_should: scope_min_should,
	})];

	if allowed_scopes.iter().any(|scope| scope == "org_shared") {
		let org_filter = Filter::all([
			Condition::matches("project_id", crate::access::ORG_PROJECT_ID.to_string()),
			Condition::matches("scope", "org_shared".to_string()),
		]);

		project_or_org_branches.push(Condition::from(org_filter));
	}

	Filter {
		must: vec![
			Condition::matches("tenant_id", tenant_id.to_string()),
			Condition::matches("status", "active".to_string()),
		],
		should: Vec::new(),
		must_not: Vec::new(),
		min_should: Some(MinShould { min_count: 1, conditions: project_or_org_branches }),
	}
}

fn doc_read_allowed(
	requester_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<SharedSpaceGrantKey>,
	owner_agent_id: &str,
	scope: &str,
) -> bool {
	if !allowed_scopes.iter().any(|s| s == scope) {
		return false;
	}
	if scope == "agent_private" {
		return owner_agent_id == requester_agent_id;
	}
	if owner_agent_id == requester_agent_id {
		return true;
	}

	shared_grants.contains(&SharedSpaceGrantKey {
		scope: scope.to_string(),
		space_owner_agent_id: owner_agent_id.to_string(),
	})
}

fn parse_scored_point_uuid_id(point: &qdrant_client::qdrant::ScoredPoint) -> Result<Uuid> {
	use qdrant_client::qdrant::point_id::PointIdOptions;

	let id = point
		.id
		.as_ref()
		.ok_or_else(|| Error::Qdrant { message: "Qdrant returned item without id.".to_string() })?;

	match id.point_id_options.as_ref() {
		Some(PointIdOptions::Uuid(s)) => Uuid::parse_str(s.as_str())
			.map_err(|_| Error::Qdrant { message: "Qdrant returned invalid uuid id.".to_string() }),
		Some(other) => Err(Error::Qdrant {
			message: format!("Qdrant returned unsupported id type: {other:?}."),
		}),
		None => Err(Error::Qdrant { message: "Qdrant returned item with missing id.".to_string() }),
	}
}

#[derive(Clone, Debug, sqlx::FromRow)]
struct DocSearchRow {
	chunk_id: Uuid,
	doc_id: Uuid,
	scope: String,
	project_id: String,
	agent_id: String,
	updated_at: OffsetDateTime,
	content_hash: String,
	chunk_hash: String,
	chunk_text: String,
}

async fn load_doc_search_rows(
	executor: impl sqlx::PgExecutor<'_>,
	tenant_id: &str,
	project_id: &str,
	chunk_ids: &[Uuid],
) -> Result<HashMap<Uuid, DocSearchRow>> {
	if chunk_ids.is_empty() {
		return Ok(HashMap::new());
	}

	let rows: Vec<DocSearchRow> = sqlx::query_as(
		"\
SELECT
	c.chunk_id,
	c.doc_id,
	d.scope,
	d.project_id,
	d.agent_id,
	d.updated_at,
	d.content_hash,
	c.chunk_hash,
	c.chunk_text
FROM doc_chunks c
JOIN doc_documents d ON d.doc_id = c.doc_id
WHERE c.chunk_id = ANY($1)
  AND d.tenant_id = $2
  AND d.status = 'active'
  AND (
    d.project_id = $3
    OR (d.project_id = $4 AND d.scope = 'org_shared')
  )",
	)
	.bind(chunk_ids)
	.bind(tenant_id)
	.bind(project_id)
	.bind(crate::access::ORG_PROJECT_ID)
	.fetch_all(executor)
	.await?;
	let mut map = HashMap::with_capacity(rows.len());
	for row in rows {
		map.insert(row.chunk_id, row);
	}

	Ok(map)
}

fn truncate_bytes(text: &str, max: usize) -> String {
	if text.len() <= max {
		return text.to_string();
	}
	let mut cut = max;
	while cut > 0 && !text.is_char_boundary(cut) {
		cut -= 1;
	}
	text.get(0..cut).unwrap_or("").to_string()
}

fn locate_quote(text: &str, quote: &TextQuoteSelector) -> Option<(usize, usize)> {
	let prefix = quote.prefix.as_deref().unwrap_or("");
	let suffix = quote.suffix.as_deref().unwrap_or("");

	for (start, _) in text.match_indices(quote.exact.as_str()) {
		let end = start + quote.exact.len();
		if !text[..start].ends_with(prefix) {
			continue;
		}
		if !text[end..].starts_with(suffix) {
			continue;
		}

		return Some((start, end));
	}

	None
}

fn bounded_window(
	match_start: usize,
	match_end: usize,
	text: &str,
	max_bytes: usize,
) -> (usize, usize) {
	let len = text.len();
	let match_center = match_start.saturating_add(match_end.saturating_sub(match_start) / 2);
	let half = max_bytes / 2;
	let mut start = match_center.saturating_sub(half);
	let mut end = (start + max_bytes).min(len);

	if end - start < max_bytes && start > 0 {
		start = start.saturating_sub(max_bytes - (end - start));
	}

	while start < len && !text.is_char_boundary(start) {
		start += 1;
	}
	while end > start && !text.is_char_boundary(end) {
		end -= 1;
	}

	(start, end)
}
