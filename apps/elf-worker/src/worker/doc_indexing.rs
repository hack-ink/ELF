use crate::worker::{
	self, BM25_MODEL, BM25_VECTOR_NAME, Condition, DENSE_VECTOR_NAME, Db, DeletePointsBuilder,
	DocChunkIndexRow, DocIndexingOutboxEntry, Document, Error, Filter, HashMap, Payload,
	PointStruct, Result, ToString, UpsertPointsBuilder, Uuid, Value, Vector, WorkerState, docs,
	embedding, slice,
};

pub(super) async fn fetch_doc_chunk_index_row(
	db: &Db,
	chunk_id: Uuid,
) -> Result<Option<DocChunkIndexRow>> {
	let row = sqlx::query_as::<_, DocChunkIndexRow>(
		r#"
SELECT
	d.doc_id,
	d.tenant_id,
	d.project_id,
	d.agent_id,
	d.scope,
	d.doc_type,
	d.status,
	d.created_at,
	d.updated_at,
	d.content_hash,
	COALESCE(d.source_ref, '{}'::jsonb) AS source_ref,
	c.chunk_id,
	c.chunk_index,
	c.start_offset,
	c.end_offset,
	c.chunk_text,
	c.chunk_hash
FROM doc_chunks c
JOIN doc_documents d ON d.doc_id = c.doc_id
WHERE c.chunk_id = $1
LIMIT 1"#,
	)
	.bind(chunk_id)
	.fetch_optional(&db.pool)
	.await?;

	Ok(row)
}

pub(super) async fn handle_doc_upsert(
	state: &WorkerState,
	job: &DocIndexingOutboxEntry,
) -> Result<()> {
	let row = fetch_doc_chunk_index_row(&state.db, job.chunk_id).await?;
	let Some(row) = row else {
		tracing::info!(
			outbox_id = %job.outbox_id,
			doc_id = %job.doc_id,
			chunk_id = %job.chunk_id,
			"Doc chunk missing for outbox job. Marking done."
		);

		return Ok(());
	};

	if !row.status.eq_ignore_ascii_case("active") {
		tracing::info!(
			outbox_id = %job.outbox_id,
			doc_id = %row.doc_id,
			chunk_id = %row.chunk_id,
			"Doc inactive. Skipping index."
		);

		return Ok(());
	}

	let vectors = embedding::embed(&state.embedding, slice::from_ref(&row.chunk_text))
		.await
		.map_err(|err| Error::Message(err.to_string()))?;
	let vector = vectors
		.first()
		.ok_or_else(|| Error::Validation("Embedding provider returned no vectors.".to_string()))?;

	worker::validate_vector_dim(vector, state.docs_qdrant.vector_dim)?;

	{
		let vec_text = worker::format_vector_text(vector);
		let mut tx = state.db.pool.begin().await?;

		docs::insert_doc_chunk_embedding(
			&mut *tx,
			row.chunk_id,
			&job.embedding_version,
			vector.len() as i32,
			vec_text.as_str(),
		)
		.await?;

		tx.commit().await?;
	}

	upsert_qdrant_doc_chunk(state, &row, &job.embedding_version, vector).await?;

	Ok(())
}

pub(super) async fn handle_doc_delete(
	state: &WorkerState,
	job: &DocIndexingOutboxEntry,
) -> Result<()> {
	let filter = Filter::must([Condition::matches("chunk_id", job.chunk_id.to_string())]);
	let delete =
		DeletePointsBuilder::new(state.docs_qdrant.collection.clone()).points(filter).wait(true);

	state.docs_qdrant.client.delete_points(delete).await?;

	Ok(())
}

pub(super) async fn upsert_qdrant_doc_chunk(
	state: &WorkerState,
	row: &DocChunkIndexRow,
	embedding_version: &str,
	vec: &[f32],
) -> Result<()> {
	let (doc_ts, thread_id, domain, repo) =
		worker::project_doc_ref_fields(&row.source_ref, row.created_at, row.doc_type.as_str())?;
	let mut payload = Payload::new();

	payload.insert("doc_id", row.doc_id.to_string());
	payload.insert("chunk_id", row.chunk_id.to_string());
	payload.insert("chunk_index", row.chunk_index as i64);
	payload.insert("start_offset", row.start_offset as i64);
	payload.insert("end_offset", row.end_offset as i64);
	payload.insert("tenant_id", row.tenant_id.clone());
	payload.insert("project_id", row.project_id.clone());
	payload.insert("agent_id", row.agent_id.clone());
	payload.insert("scope", row.scope.clone());
	payload.insert("doc_type", row.doc_type.clone());
	payload.insert("status", row.status.clone());

	let updated_at = worker::format_timestamp(row.updated_at)?;

	payload.insert("updated_at", Value::String(updated_at));
	payload.insert("doc_ts", Value::String(doc_ts));

	if let Some(value) = thread_id {
		payload.insert("thread_id", Value::String(value));
	}
	if let Some(value) = domain {
		payload.insert("domain", Value::String(value));
	}
	if let Some(value) = repo {
		payload.insert("repo", Value::String(value));
	}

	payload.insert("embedding_version", embedding_version.to_string());
	payload.insert("content_hash", row.content_hash.clone());
	payload.insert("chunk_hash", row.chunk_hash.clone());

	let mut vector_map = HashMap::new();

	vector_map.insert(DENSE_VECTOR_NAME.to_string(), Vector::from(vec.to_vec()));
	vector_map.insert(
		BM25_VECTOR_NAME.to_string(),
		Vector::from(Document::new(row.chunk_text.clone(), BM25_MODEL)),
	);

	let point = PointStruct::new(row.chunk_id.to_string(), vector_map, payload);
	let upsert =
		UpsertPointsBuilder::new(state.docs_qdrant.collection.clone(), vec![point]).wait(true);

	state.docs_qdrant.client.upsert_points(upsert).await?;

	Ok(())
}
