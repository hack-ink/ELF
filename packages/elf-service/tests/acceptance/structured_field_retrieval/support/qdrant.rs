use std::collections::HashMap;

use qdrant_client::{
	Payload,
	qdrant::{Document, PointStruct, UpsertPointsBuilder, Vector},
};
use serde_json::Value;
use uuid::Uuid;

use elf_service::ElfService;
use elf_storage::qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME};

pub(crate) struct UpsertPointArgs<'a> {
	pub(crate) chunk_id: Uuid,
	pub(crate) note_id: Uuid,
	pub(crate) chunk_index: i32,
	pub(crate) start_offset: i32,
	pub(crate) end_offset: i32,
	pub(crate) text: &'a str,
	pub(crate) dense: Vec<f32>,
}

pub(crate) async fn upsert_point(service: &ElfService, args: UpsertPointArgs<'_>) {
	let payload = build_payload(
		args.note_id,
		args.chunk_id,
		args.chunk_index,
		args.start_offset,
		args.end_offset,
	);
	let vectors = build_vectors(args.text, args.dense);
	let point = PointStruct::new(args.chunk_id.to_string(), vectors, payload);

	service
		.qdrant
		.client
		.upsert_points(
			UpsertPointsBuilder::new(service.qdrant.collection.clone(), vec![point]).wait(true),
		)
		.await
		.expect("Failed to upsert Qdrant point.");
}

fn build_payload(
	note_id: Uuid,
	chunk_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
) -> Payload {
	let mut payload = Payload::new();

	payload.insert("note_id", note_id.to_string());
	payload.insert("chunk_id", chunk_id.to_string());
	payload.insert("chunk_index", Value::from(chunk_index));
	payload.insert("start_offset", Value::from(start_offset));
	payload.insert("end_offset", Value::from(end_offset));
	payload.insert("tenant_id", "t");
	payload.insert("project_id", "p");
	payload.insert("agent_id", "a");
	payload.insert("scope", "agent_private");
	payload.insert("status", "active");

	payload
}

fn build_vectors(text: &str, dense: Vec<f32>) -> HashMap<String, Vector> {
	let mut vectors = HashMap::new();

	vectors.insert(DENSE_VECTOR_NAME.to_string(), Vector::from(dense));
	vectors.insert(
		BM25_VECTOR_NAME.to_string(),
		Vector::from(Document::new(text.to_string(), BM25_MODEL)),
	);

	vectors
}
