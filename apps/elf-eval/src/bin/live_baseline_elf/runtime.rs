use color_eyre::Result;

use crate::{
	AGENT_ID, Arc, BTreeMap, BaselineRuntime, ChunkingConfig, Db, ElfService, EmbeddingMode,
	FailedOutboxJob, Instant, JoinSet, PROJECT_ID, PayloadLevel, QdrantStore, QueryCase,
	QueryResult, SearchRequest, TENANT_ID, Uuid, Value, WorkerRunEvidence, WorkerState, env, eyre,
	worker,
};

pub(super) fn worker_max_iterations(note_count: usize) -> usize {
	env::var("ELF_BASELINE_WORKER_MAX_ITERATIONS")
		.ok()
		.and_then(|value| value.parse::<usize>().ok())
		.unwrap_or_else(|| note_count.saturating_mul(3).saturating_add(32))
}

pub(super) async fn build_service(runtime: &BaselineRuntime) -> Result<ElfService> {
	let cfg = crate::runtime_config(runtime)?;
	let embedding_mode = crate::embedding_mode()?;
	let vector_dim = cfg.storage.qdrant.vector_dim;
	let db = Db::connect(&cfg.storage.postgres).await?;

	db.ensure_schema(cfg.storage.qdrant.vector_dim).await?;

	let qdrant = QdrantStore::new(&cfg.storage.qdrant)?;

	qdrant.ensure_collection().await?;

	if embedding_mode == EmbeddingMode::Provider {
		Ok(ElfService::new(cfg, db, qdrant))
	} else {
		Ok(ElfService::with_providers(cfg, db, qdrant, crate::deterministic_providers(vector_dim)))
	}
}

pub(super) async fn build_worker_state(runtime: &BaselineRuntime) -> Result<WorkerState> {
	let cfg = crate::runtime_config(runtime)?;
	let db = Db::connect(&cfg.storage.postgres).await?;

	db.ensure_schema(cfg.storage.qdrant.vector_dim).await?;

	let qdrant = QdrantStore::new(&cfg.storage.qdrant)?;

	qdrant.ensure_collection().await?;

	let docs_qdrant =
		QdrantStore::new_with_collection(&cfg.storage.qdrant, &cfg.storage.qdrant.docs_collection)?;

	docs_qdrant.ensure_collection().await?;

	let tokenizer = elf_chunking::load_tokenizer(&cfg.chunking.tokenizer_repo)
		.map_err(|err| eyre::eyre!("Failed to load tokenizer for live baseline worker: {err}"))?;
	let chunking = ChunkingConfig {
		max_tokens: cfg.chunking.max_tokens,
		overlap_tokens: cfg.chunking.overlap_tokens,
	};

	Ok(WorkerState {
		db,
		qdrant,
		docs_qdrant,
		embedding: cfg.providers.embedding,
		chunking,
		tokenizer,
	})
}

pub(super) async fn run_worker_until_indexed(
	runtime: &BaselineRuntime,
	service: &ElfService,
	note_ids: &[Uuid],
	label: &str,
) -> Result<WorkerRunEvidence> {
	let concurrency = crate::worker_concurrency();
	let mut states = Vec::with_capacity(concurrency);

	for _ in 0..concurrency {
		states.push(Arc::new(build_worker_state(runtime).await?));
	}

	let before = outbox_status_counts(service, note_ids).await?;
	let max_iterations = worker_max_iterations(note_ids.len());
	let mut iterations = 0_usize;

	while iterations < max_iterations {
		let after = outbox_status_counts(service, note_ids).await?;

		if crate::outbox_done(&after, note_ids.len()) {
			let (chunk_rows, chunk_embedding_rows) = chunk_counts(service, note_ids).await?;
			let failed_jobs = failed_outbox_jobs(service, note_ids).await?;

			return Ok(WorkerRunEvidence {
				label: label.to_string(),
				expected_note_count: note_ids.len(),
				concurrency,
				iterations,
				before,
				after,
				chunk_rows,
				chunk_embedding_rows,
				failed_jobs,
			});
		}

		let mut set = JoinSet::new();

		for state in &states {
			let state = Arc::clone(state);

			set.spawn(async move {
				worker::process_once(&state)
					.await
					.map_err(|err| eyre::eyre!("Worker process_once failed: {err}"))
			});
		}

		while let Some(joined) = set.join_next().await {
			joined??;
		}

		iterations = iterations.saturating_add(concurrency);
	}

	let after = outbox_status_counts(service, note_ids).await?;
	let (chunk_rows, chunk_embedding_rows) = chunk_counts(service, note_ids).await?;
	let failed_jobs = failed_outbox_jobs(service, note_ids).await?;

	Ok(WorkerRunEvidence {
		label: label.to_string(),
		expected_note_count: note_ids.len(),
		concurrency,
		iterations,
		before,
		after,
		chunk_rows,
		chunk_embedding_rows,
		failed_jobs,
	})
}

pub(super) async fn outbox_status_counts(
	service: &ElfService,
	note_ids: &[Uuid],
) -> Result<BTreeMap<String, i64>> {
	if note_ids.is_empty() {
		return Ok(BTreeMap::new());
	}

	let rows = sqlx::query_as::<_, (String, i64)>(
		"\
SELECT status, COUNT(*)::bigint
FROM indexing_outbox
WHERE note_id = ANY($1)
GROUP BY status
ORDER BY status",
	)
	.bind(note_ids)
	.fetch_all(&service.db.pool)
	.await?;

	Ok(rows.into_iter().collect())
}

pub(super) async fn chunk_counts(service: &ElfService, note_ids: &[Uuid]) -> Result<(i64, i64)> {
	if note_ids.is_empty() {
		return Ok((0, 0));
	}

	let chunk_rows = sqlx::query_scalar::<_, i64>(
		"\
SELECT COUNT(*)::bigint
FROM memory_note_chunks
WHERE note_id = ANY($1)",
	)
	.bind(note_ids)
	.fetch_one(&service.db.pool)
	.await?;
	let chunk_embedding_rows = sqlx::query_scalar::<_, i64>(
		"\
SELECT COUNT(*)::bigint
FROM memory_note_chunks c
JOIN note_chunk_embeddings e ON e.chunk_id = c.chunk_id
WHERE c.note_id = ANY($1)",
	)
	.bind(note_ids)
	.fetch_one(&service.db.pool)
	.await?;

	Ok((chunk_rows, chunk_embedding_rows))
}

pub(super) async fn failed_outbox_jobs(
	service: &ElfService,
	note_ids: &[Uuid],
) -> Result<Vec<FailedOutboxJob>> {
	if note_ids.is_empty() {
		return Ok(Vec::new());
	}

	let rows = sqlx::query_as::<_, (Uuid, Option<String>, String, i32, Option<String>)>(
		"\
SELECT o.note_id, n.key, o.op, o.attempts, o.last_error
FROM indexing_outbox o
LEFT JOIN memory_notes n ON n.note_id = o.note_id
WHERE o.note_id = ANY($1)
	AND o.status = 'FAILED'
ORDER BY n.key NULLS LAST, o.note_id",
	)
	.bind(note_ids)
	.fetch_all(&service.db.pool)
	.await?;

	Ok(rows
		.into_iter()
		.map(|(note_id, note_key, op, attempts, last_error)| FailedOutboxJob {
			note_id,
			note_key,
			op,
			attempts,
			last_error,
		})
		.collect())
}

pub(super) async fn run_queries(
	service: &ElfService,
	queries: Vec<QueryCase>,
) -> Result<Vec<QueryResult>> {
	let mut out = Vec::with_capacity(queries.len());

	for case in queries {
		out.push(run_single_query(service, case).await?);
	}

	Ok(out)
}

pub(super) async fn run_single_query(service: &ElfService, case: QueryCase) -> Result<QueryResult> {
	let top_k = env::var("ELF_BASELINE_TOP_K")
		.ok()
		.and_then(|value| value.parse::<u32>().ok())
		.unwrap_or(10);
	let started_at = Instant::now();
	let response = service
		.search_raw(SearchRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: AGENT_ID.to_string(),
			token_id: None,
			payload_level: PayloadLevel::L2,
			read_profile: "private_only".to_string(),
			query: case.query.clone(),
			top_k: Some(top_k),
			candidate_k: Some(top_k.max(20).saturating_mul(4)),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await?;
	let latency_ms = started_at.elapsed().as_secs_f64() * 1_000.0;
	let top = response.items.first();
	let top_text = top.map(|item| item.snippet.clone()).unwrap_or_default();
	let matched_terms = case
		.expected_terms
		.iter()
		.filter(|term| crate::contains_case_insensitive(&top_text, term))
		.cloned()
		.collect::<Vec<_>>();
	let top_key = top.and_then(|item| item.key.clone());
	let expected_docs = crate::expected_docs_for_case(&case);
	let matched_doc = top_key
		.as_deref()
		.and_then(|key| expected_docs.iter().find(|doc| crate::key_for_doc(doc) == key));
	let top_evidence_id = top.and_then(|item| {
		item.source_ref.get("document").and_then(Value::as_str).map(crate::evidence_id_for_doc)
	});
	let matched_evidence_id = matched_doc.map(|doc| crate::evidence_id_for_doc(doc));
	let matched = matched_terms.len() == case.expected_terms.len() || matched_doc.is_some();
	let expected_evidence_ids = if case.expected_evidence_ids.is_empty() {
		vec![crate::evidence_id_for_doc(&case.expected_doc)]
	} else {
		case.expected_evidence_ids.clone()
	};
	let allowed_alternate_evidence_ids = if case.allowed_alternate_evidence_ids.is_empty() {
		case.allowed_alternate_docs.iter().map(|doc| crate::evidence_id_for_doc(doc)).collect()
	} else {
		case.allowed_alternate_evidence_ids.clone()
	};

	Ok(QueryResult {
		id: case.id,
		task: case.task,
		trace_id: response.trace_id,
		query: case.query,
		expected_doc: case.expected_doc,
		allowed_alternate_docs: case.allowed_alternate_docs,
		expected_terms: case.expected_terms,
		expected_evidence_ids,
		allowed_alternate_evidence_ids,
		matched,
		matched_terms,
		top_evidence_id,
		matched_evidence_id,
		top_note_key: top_key,
		top_snippet: top.map(|item| item.snippet.clone()),
		latency_ms,
		returned_count: response.items.len(),
	})
}
