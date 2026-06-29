use crate::worker::{
	self, BM25_MODEL, BM25_VECTOR_NAME, ChunkRecord, Condition, DENSE_VECTOR_NAME,
	DeletePointsBuilder, Document, Filter, HashMap, MemoryNote, Payload, PointStruct, Result,
	ToString, UpsertPointsBuilder, Uuid, Value, Vector, WorkerState,
};

pub(super) async fn delete_note_points(state: &WorkerState, note_id: Uuid) -> Result<()> {
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

pub(super) async fn replace_chunks(
	state: &WorkerState,
	note: &MemoryNote,
	embedding_version: &str,
	records: &[ChunkRecord],
	vectors: &[Vec<f32>],
) -> Result<()> {
	delete_note_points(state, note.note_id).await?;

	upsert_chunks(state, note, embedding_version, records, vectors).await
}

fn build_chunk_point(
	note: &MemoryNote,
	embedding_version: &str,
	record: &ChunkRecord,
	vec: &[f32],
) -> Result<PointStruct> {
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

	Ok(PointStruct::new(record.chunk_id.to_string(), vector_map, payload))
}

async fn upsert_chunks(
	state: &WorkerState,
	note: &MemoryNote,
	embedding_version: &str,
	records: &[ChunkRecord],
	vectors: &[Vec<f32>],
) -> Result<()> {
	let mut points = Vec::with_capacity(records.len());

	for (record, vec) in records.iter().zip(vectors.iter()) {
		points.push(build_chunk_point(note, embedding_version, record, vec)?);
	}

	let upsert = UpsertPointsBuilder::new(state.qdrant.collection.clone(), points).wait(true);

	state.qdrant.client.upsert_points(upsert).await?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use qdrant_client::qdrant::{value, vectors};

	use crate::worker::{
		BM25_VECTOR_NAME, ChunkRecord, DENSE_VECTOR_NAME, MemoryNote, OffsetDateTime, Uuid, Value,
		note_indexing::qdrant_points,
	};

	fn test_note() -> MemoryNote {
		MemoryNote {
			note_id: Uuid::parse_str("018f0c9d-2d1e-7b8f-8000-000000000001")
				.expect("note id should parse"),
			tenant_id: "tenant-a".to_string(),
			project_id: "project-a".to_string(),
			agent_id: "agent-a".to_string(),
			scope: "project".to_string(),
			r#type: "fact".to_string(),
			key: None,
			text: "Important deployment note.".to_string(),
			importance: 0.7,
			confidence: 0.9,
			status: "active".to_string(),
			created_at: OffsetDateTime::from_unix_timestamp(1_700_000_000)
				.expect("created timestamp should parse"),
			updated_at: OffsetDateTime::from_unix_timestamp(1_700_000_123)
				.expect("updated timestamp should parse"),
			expires_at: None,
			embedding_version: "embed-v1".to_string(),
			source_ref: Value::Null,
			hit_count: 0,
			last_hit_at: None,
		}
	}

	fn test_record() -> ChunkRecord {
		ChunkRecord {
			chunk_id: Uuid::parse_str("018f0c9d-2d1e-7b8f-8000-000000000002")
				.expect("chunk id should parse"),
			chunk_index: 2,
			start_offset: 10,
			end_offset: 39,
			text: "Important deployment note.".to_string(),
		}
	}

	fn payload_string(point: &qdrant_client::qdrant::PointStruct, key: &str) -> String {
		let value = point.payload.get(key).expect("payload key should exist");
		let Some(value::Kind::StringValue(value)) = value.kind.as_ref() else {
			panic!("payload key should be a string")
		};

		value.clone()
	}

	fn payload_integer(point: &qdrant_client::qdrant::PointStruct, key: &str) -> i64 {
		let value = point.payload.get(key).expect("payload key should exist");
		let Some(value::Kind::IntegerValue(value)) = value.kind.as_ref() else {
			panic!("payload key should be an integer")
		};

		*value
	}

	#[test]
	fn build_chunk_point_preserves_payload_and_named_vectors() {
		let note = test_note();
		let record = test_record();
		let point = qdrant_points::build_chunk_point(&note, "embed-v2", &record, &[0.1, 0.2, 0.3])
			.expect("point should build");

		assert_eq!(payload_string(&point, "note_id"), note.note_id.to_string());
		assert_eq!(payload_string(&point, "chunk_id"), record.chunk_id.to_string());
		assert_eq!(payload_integer(&point, "chunk_index"), 2);
		assert_eq!(payload_string(&point, "tenant_id"), "tenant-a");
		assert_eq!(payload_string(&point, "embedding_version"), "embed-v2");

		let key = point.payload.get("key").expect("key payload should exist");

		assert!(matches!(key.kind.as_ref(), Some(value::Kind::NullValue(_))));

		let vectors = point.vectors.expect("point should include vectors");
		let Some(vectors::VectorsOptions::Vectors(named_vectors)) = vectors.vectors_options else {
			panic!("point should use named vectors")
		};

		assert!(named_vectors.vectors.contains_key(DENSE_VECTOR_NAME));
		assert!(named_vectors.vectors.contains_key(BM25_VECTOR_NAME));
	}
}
