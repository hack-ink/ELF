use super::*;

pub(super) async fn run_doc_fusion_query(
	client: &Qdrant,
	collection: &str,
	query_text: &str,
	vector: &[f32],
	filter: &Filter,
	sparse_mode: DocsSparseMode,
	candidate_k: u32,
) -> Result<Vec<ScoredPoint>> {
	let sparse_enabled = docs_search_sparse_enabled(sparse_mode, query_text);
	let dense_prefetch = PrefetchQueryBuilder::default()
		.query(Query::new_nearest(vector.to_vec()))
		.using(DENSE_VECTOR_NAME)
		.filter(filter.clone())
		.limit(candidate_k as u64);
	let mut search = QueryPointsBuilder::new(collection.to_string());

	search = search.add_prefetch(dense_prefetch);

	if sparse_enabled {
		let bm25_prefetch = PrefetchQueryBuilder::default()
			.query(Query::new_nearest(Document::new(query_text.to_string(), BM25_MODEL)))
			.using(BM25_VECTOR_NAME)
			.filter(filter.clone())
			.limit(candidate_k as u64);

		search = search.add_prefetch(bm25_prefetch);
	}

	let search = search.with_payload(false).query(Fusion::Rrf).limit(candidate_k as u64);
	let response =
		client.query(search).await.map_err(|err| Error::Qdrant { message: err.to_string() })?;

	Ok(response.result)
}

pub(super) async fn load_doc_search_rows(
	executor: impl PgExecutor<'_>,
	tenant_id: &str,
	project_id: &str,
	status: &str,
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
	d.doc_type,
	d.project_id,
	d.agent_id,
	d.updated_at,
	d.content_hash,
	c.chunk_hash,
	c.start_offset,
	c.end_offset,
	c.chunk_text
FROM doc_chunks c
JOIN doc_documents d ON d.doc_id = c.doc_id
WHERE c.chunk_id = ANY($1)
  AND d.tenant_id = $2
  AND d.status = $4
  AND (
    d.project_id = $3
    OR (d.project_id = $5 AND d.scope = 'org_shared')
  )",
	)
	.bind(chunk_ids)
	.bind(tenant_id)
	.bind(project_id)
	.bind(status)
	.bind(ORG_PROJECT_ID)
	.fetch_all(executor)
	.await?;
	let mut map = HashMap::with_capacity(rows.len());

	for row in rows {
		map.insert(row.chunk_id, row);
	}

	Ok(map)
}
