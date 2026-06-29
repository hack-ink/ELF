use crate::worker::{
	self, BM25_MODEL, BM25_VECTOR_NAME, ChunkRecord, Condition, DENSE_VECTOR_NAME, Db,
	DeletePointsBuilder, Document, Error, Filter, HashMap, IndexingOutboxEntry, MemoryNote,
	NoteFieldRow, OffsetDateTime, Payload, PgExecutor, PointStruct, Result, ToString,
	UpsertPointsBuilder, Uuid, Value, Vector, WorkerState, embedding, queries,
};

pub(super) async fn handle_upsert(state: &WorkerState, job: &IndexingOutboxEntry) -> Result<()> {
	let note = fetch_note(&state.db, job.note_id).await?;
	let Some(note) = note else {
		tracing::info!(
			outbox_id = %job.outbox_id,
			note_id = %job.note_id,
			"Note missing for outbox job. Marking done."
		);

		return Ok(());
	};
	let now = OffsetDateTime::now_utc();

	if !worker::note_is_active(&note, now) {
		tracing::info!(
			outbox_id = %job.outbox_id,
			note_id = %job.note_id,
			"Note inactive or expired. Skipping index."
		);

		return Ok(());
	}

	let fields = fetch_note_fields(&state.db, note.note_id).await?;
	let chunks = elf_chunking::split_text(&note.text, &state.chunking, &state.tokenizer);

	if chunks.is_empty() {
		return Err(Error::Validation("Chunking produced no chunks.".to_string()));
	}

	let records = worker::build_chunk_records(note.note_id, &chunks)?;
	let chunk_texts: Vec<String> = records.iter().map(|record| record.text.clone()).collect();
	let field_texts: Vec<String> = fields.iter().map(|field| field.text.clone()).collect();
	let mut embed_inputs = Vec::with_capacity(chunk_texts.len() + field_texts.len());

	embed_inputs.extend(chunk_texts);
	embed_inputs.extend(field_texts);

	let vectors = embedding::embed(&state.embedding, &embed_inputs)
		.await
		.map_err(|err| Error::Message(err.to_string()))?;

	if vectors.len() != records.len() + fields.len() {
		return Err(Error::Validation(format!(
			"Embedding provider returned {} vectors for {} items.",
			vectors.len(),
			records.len() + fields.len()
		)));
	}

	let (chunk_vectors, field_vectors) = vectors.split_at(records.len());

	for vector in chunk_vectors.iter().chain(field_vectors.iter()) {
		worker::validate_vector_dim(vector, state.qdrant.vector_dim)?;
	}

	{
		let mut tx = state.db.pool.begin().await?;

		queries::delete_note_chunks(&mut *tx, note.note_id).await?;

		for record in &records {
			queries::insert_note_chunk(
				&mut *tx,
				record.chunk_id,
				note.note_id,
				record.chunk_index,
				record.start_offset,
				record.end_offset,
				record.text.as_str(),
				&job.embedding_version,
			)
			.await?;
		}
		for (record, vector) in records.iter().zip(chunk_vectors.iter()) {
			let vec_text = worker::format_vector_text(vector);

			queries::insert_note_chunk_embedding(
				&mut *tx,
				record.chunk_id,
				&job.embedding_version,
				vector.len() as i32,
				vec_text.as_str(),
			)
			.await?;
		}

		let pooled = worker::mean_pool(chunk_vectors)
			.ok_or_else(|| Error::Message("Cannot pool empty chunk vectors.".to_string()))?;

		worker::validate_vector_dim(&pooled, state.qdrant.vector_dim)?;

		insert_embedding_tx(
			&mut *tx,
			note.note_id,
			&job.embedding_version,
			pooled.len() as i32,
			&pooled,
		)
		.await?;

		for (field, vector) in fields.iter().zip(field_vectors.iter()) {
			insert_note_field_embedding_tx(
				&mut *tx,
				field.field_id,
				&job.embedding_version,
				vector.len() as i32,
				vector,
			)
			.await?;
		}

		tx.commit().await?;
	}

	delete_qdrant_note_points(state, note.note_id).await?;
	upsert_qdrant_chunks(state, &note, &job.embedding_version, &records, chunk_vectors).await?;

	Ok(())
}

pub(super) async fn handle_delete(state: &WorkerState, job: &IndexingOutboxEntry) -> Result<()> {
	delete_qdrant_note_points(state, job.note_id).await?;

	Ok(())
}

pub(super) async fn fetch_note(db: &Db, note_id: Uuid) -> Result<Option<MemoryNote>> {
	let note = sqlx::query_as::<_, MemoryNote>("SELECT * FROM memory_notes WHERE note_id = $1")
		.bind(note_id)
		.fetch_optional(&db.pool)
		.await?;

	Ok(note)
}

pub(super) async fn fetch_note_fields(db: &Db, note_id: Uuid) -> Result<Vec<NoteFieldRow>> {
	let rows = sqlx::query_as::<_, NoteFieldRow>(
		"\
SELECT field_id, text
FROM memory_note_fields
WHERE note_id = $1
ORDER BY field_kind ASC, item_index ASC",
	)
	.bind(note_id)
	.fetch_all(&db.pool)
	.await?;

	Ok(rows)
}

pub(super) async fn insert_embedding_tx<'e, E>(
	executor: E,
	note_id: Uuid,
	embedding_version: &str,
	embedding_dim: i32,
	vec: &[f32],
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	let vec_text = worker::format_vector_text(vec);

	sqlx::query(
		"\
INSERT INTO note_embeddings (
	note_id,
	embedding_version,
	embedding_dim,
	vec
)
VALUES ($1, $2, $3, $4::text::vector)
ON CONFLICT (note_id, embedding_version) DO UPDATE
SET
	embedding_dim = EXCLUDED.embedding_dim,
	vec = EXCLUDED.vec,
	created_at = now()",
	)
	.bind(note_id)
	.bind(embedding_version)
	.bind(embedding_dim)
	.bind(vec_text.as_str())
	.execute(executor)
	.await?;

	Ok(())
}

pub(super) async fn insert_note_field_embedding_tx<'e, E>(
	executor: E,
	field_id: Uuid,
	embedding_version: &str,
	embedding_dim: i32,
	vec: &[f32],
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	let vec_text = worker::format_vector_text(vec);

	sqlx::query(
		"\
INSERT INTO note_field_embeddings (
	field_id,
	embedding_version,
	embedding_dim,
	vec
)
VALUES ($1, $2, $3, $4::text::vector)
ON CONFLICT (field_id, embedding_version) DO UPDATE
SET
	embedding_dim = EXCLUDED.embedding_dim,
	vec = EXCLUDED.vec,
	created_at = now()",
	)
	.bind(field_id)
	.bind(embedding_version)
	.bind(embedding_dim)
	.bind(vec_text.as_str())
	.execute(executor)
	.await?;

	Ok(())
}

pub(super) async fn delete_qdrant_note_points(state: &WorkerState, note_id: Uuid) -> Result<()> {
	let filter = Filter::must([Condition::matches("note_id", note_id.to_string())]);
	let delete =
		DeletePointsBuilder::new(state.qdrant.collection.clone()).points(filter).wait(true);

	match state.qdrant.client.delete_points(delete).await {
		Ok(_) => {},
		Err(err) =>
			if worker::is_not_found_error(&err) {
				tracing::info!(note_id = %note_id, "Qdrant points missing during delete.");
			} else {
				return Err(err.into());
			},
	}

	Ok(())
}

pub(super) async fn upsert_qdrant_chunks(
	state: &WorkerState,
	note: &MemoryNote,
	embedding_version: &str,
	records: &[ChunkRecord],
	vectors: &[Vec<f32>],
) -> Result<()> {
	let mut points = Vec::with_capacity(records.len());

	for (record, vec) in records.iter().zip(vectors.iter()) {
		let mut payload = Payload::new();

		payload.insert("note_id", note.note_id.to_string());
		payload.insert("chunk_id", record.chunk_id.to_string());
		payload.insert("chunk_index", record.chunk_index as i64);
		payload.insert("start_offset", record.start_offset as i64);
		payload.insert("end_offset", record.end_offset as i64);
		payload.insert("tenant_id", note.tenant_id.clone());
		payload.insert("project_id", note.project_id.clone());
		payload.insert("agent_id", note.agent_id.clone());
		payload.insert("scope", note.scope.clone());
		payload.insert("status", note.status.clone());
		payload.insert("type", note.r#type.clone());

		match note.key.as_ref() {
			Some(key) => payload.insert("key", key.clone()),
			None => payload.insert("key", Value::Null),
		}

		payload.insert("updated_at", Value::String(worker::format_timestamp(note.updated_at)?));
		payload.insert(
			"expires_at",
			match note.expires_at {
				Some(ts) => Value::String(worker::format_timestamp(ts)?),
				None => Value::Null,
			},
		);
		payload.insert("importance", Value::from(note.importance as f64));
		payload.insert("confidence", Value::from(note.confidence as f64));
		payload.insert("embedding_version", embedding_version.to_string());

		let mut vector_map = HashMap::new();

		vector_map.insert(DENSE_VECTOR_NAME.to_string(), Vector::from(vec.to_vec()));
		vector_map.insert(
			BM25_VECTOR_NAME.to_string(),
			Vector::from(Document::new(record.text.clone(), BM25_MODEL)),
		);

		let point = PointStruct::new(record.chunk_id.to_string(), vector_map, payload);

		points.push(point);
	}

	let upsert = UpsertPointsBuilder::new(state.qdrant.collection.clone(), points).wait(true);

	state.qdrant.client.upsert_points(upsert).await?;

	Ok(())
}
