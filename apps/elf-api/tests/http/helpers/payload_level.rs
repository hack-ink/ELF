use std::collections::HashMap;

use axum::{
	Router,
	body::{self, Body},
	http::{Request, StatusCode},
};
use qdrant_client::{
	Payload,
	qdrant::{Document, PointStruct, UpsertPointsBuilder, Vector},
};
use tower::util::ServiceExt as _;
use uuid::Uuid;

use crate::helpers::{self, TEST_AGENT_A, TEST_PROJECT_ID, TEST_TENANT_ID};
use elf_api::state::AppState;
use elf_storage::qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME};

pub(crate) async fn create_note_for_payload_level_tests(
	app: &Router,
	state: &AppState,
	text: &str,
	source_ref: serde_json::Value,
) -> Uuid {
	helpers::init_test_tracing();

	let payload = serde_json::json!({
		"scope": "agent_private",
		"notes": [{
			"type": "fact",
			"key": null,
			"text": text,
			"importance": 0.8,
			"confidence": 0.9,
			"ttl_days": null,
			"source_ref": source_ref,
		}]
	});
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/notes/ingest")
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_A)
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build note ingest request."),
		)
		.await
		.expect("Failed to call note ingest.");
	let status = response.status();
	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read note ingest response body.");

	assert_eq!(
		status,
		StatusCode::OK,
		"Unexpected note ingest status with body: {}",
		String::from_utf8_lossy(&body)
	);

	let json: serde_json::Value =
		serde_json::from_slice(&body).expect("Failed to parse note ingest response.");
	let note_id = json["results"]
		.as_array()
		.expect("Missing results array in note ingest response.")
		.first()
		.and_then(|result| result["note_id"].as_str())
		.expect("Missing note_id in note ingest response.");
	let note_id = Uuid::parse_str(note_id).expect("Invalid note_id in note ingest response.");

	index_note_for_payload_level_tests(state, note_id, text).await;

	note_id
}

pub(crate) async fn index_note_for_payload_level_tests(
	state: &AppState,
	note_id: Uuid,
	text: &str,
) {
	let chunk_id = Uuid::new_v4();
	let embedding_version = format!(
		"{}:{}:{}",
		state.service.cfg.providers.embedding.provider_id,
		state.service.cfg.providers.embedding.model,
		state.service.cfg.storage.qdrant.vector_dim
	);

	sqlx::query(
		"INSERT INTO memory_note_chunks (
			chunk_id,
			note_id,
			chunk_index,
			start_offset,
			end_offset,
			text,
			embedding_version
		) VALUES ($1, $2, $3, $4, $5, $6, $7)",
	)
	.bind(chunk_id)
	.bind(note_id)
	.bind(0_i32)
	.bind(0_i32)
	.bind(i32::try_from(text.len()).expect("Payload-level test text fits i32 offsets."))
	.bind(text)
	.bind(embedding_version.as_str())
	.execute(&state.service.db.pool)
	.await
	.expect("Failed to seed memory note chunk.");

	let mut payload = Payload::new();

	payload.insert("note_id", note_id.to_string());
	payload.insert("chunk_id", chunk_id.to_string());
	payload.insert("chunk_index", 0_i64);
	payload.insert("start_offset", 0_i64);
	payload.insert("end_offset", i64::try_from(text.len()).expect("Test text fits i64 offsets."));
	payload.insert("tenant_id", TEST_TENANT_ID);
	payload.insert("project_id", TEST_PROJECT_ID);
	payload.insert("agent_id", TEST_AGENT_A);
	payload.insert("scope", "agent_private");
	payload.insert("type", "fact");
	payload.insert("status", "active");
	payload.insert("embedding_version", embedding_version);

	let mut vectors = HashMap::new();

	vectors.insert(
		DENSE_VECTOR_NAME.to_string(),
		Vector::from(vec![0.0_f32; state.service.qdrant.vector_dim as usize]),
	);
	vectors.insert(
		BM25_VECTOR_NAME.to_string(),
		Vector::from(Document::new(text.to_string(), BM25_MODEL)),
	);

	let point = PointStruct::new(chunk_id.to_string(), vectors, payload);

	state
		.service
		.qdrant
		.client
		.upsert_points(
			UpsertPointsBuilder::new(state.service.qdrant.collection.clone(), vec![point])
				.wait(true),
		)
		.await
		.expect("Failed to seed Qdrant point.");
}

pub(crate) async fn insert_note_summary_field(state: &AppState, note_id: Uuid, summary: &str) {
	sqlx::query(
		"INSERT INTO memory_note_fields (field_id, note_id, field_kind, item_index, text) \
		 VALUES ($1, $2, $3, $4, $5)",
	)
	.bind(Uuid::new_v4())
	.bind(note_id)
	.bind("summary")
	.bind(0)
	.bind(summary)
	.execute(&state.service.db.pool)
	.await
	.expect("Failed to insert note summary field.");
}

pub(crate) async fn fetch_search_notes_for_payload_level(
	app: &Router,
	search_id: Uuid,
	note_id: Uuid,
	payload_level: &str,
) -> serde_json::Value {
	let payload = serde_json::json!({
		"note_ids": [note_id],
		"payload_level": payload_level,
		"record_hits": false,
	});
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri(format!("/v2/searches/{search_id}/notes"))
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_A)
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build search notes request."),
		)
		.await
		.expect("Failed to call search notes.");
	let status = response.status();
	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read search notes response body.");

	assert_eq!(
		status,
		StatusCode::OK,
		"Unexpected search notes response: {}",
		String::from_utf8_lossy(&body)
	);

	let json: serde_json::Value =
		serde_json::from_slice(&body).expect("Failed to parse search notes response.");

	json.get("results")
		.and_then(serde_json::Value::as_array)
		.and_then(|results| results.first())
		.and_then(|result| result.get("note"))
		.cloned()
		.expect("Expected note in search notes response.")
}

pub(crate) async fn fetch_admin_search_raw_source_ref(
	app: &Router,
	query: &str,
	payload_level: &str,
) -> serde_json::Value {
	let payload = serde_json::json!({
		"mode": "quick_find",
		"query": query,
		"top_k": 5,
		"candidate_k": 10,
		"payload_level": payload_level,
	});
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/admin/searches/raw")
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_A)
				.header("X-ELF-Read-Profile", "private_only")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build admin search raw request."),
		)
		.await
		.expect("Failed to call admin search raw.");
	let status = response.status();
	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read admin search raw response body.");

	assert_eq!(
		status,
		StatusCode::OK,
		"Unexpected admin search raw status with body: {}",
		String::from_utf8_lossy(&body)
	);

	let json: serde_json::Value =
		serde_json::from_slice(&body).expect("Failed to parse admin search raw response.");
	let item = json["items"]
		.as_array()
		.expect("Missing items in admin search raw response.")
		.first()
		.expect("Expected at least one raw search item.");

	item["source_ref"].clone()
}
