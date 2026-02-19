use std::{
	future::IntoFuture,
	sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	},
	time::{Duration, Instant},
};

use ahash::AHashMap;
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing};
use serde_json::{Map, Value};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;
use tokenizers::{Tokenizer, models::wordlevel::WordLevel};
use tokio::{
	net::TcpListener,
	sync::{oneshot, oneshot::Sender},
};
use uuid::Uuid;

use crate::acceptance::{SpyExtractor, StubEmbedding, StubRerank};
use elf_config::EmbeddingProviderConfig;
use elf_service::{AddNoteInput, AddNoteRequest, Providers};
use elf_storage::{db::Db, qdrant::QdrantStore};
use elf_worker::worker;

#[derive(FromRow)]
struct OutboxRow {
	status: String,
	attempts: i32,
	last_error: Option<String>,
}

async fn wait_for_status(
	pool: &PgPool,
	note_id: Uuid,
	status: &str,
	timeout: Duration,
) -> Option<OutboxRow> {
	let deadline = Instant::now() + timeout;

	loop {
		let row: Option<OutboxRow> = sqlx::query_as::<_, OutboxRow>(
			"\
SELECT
	status,
	attempts,
	last_error
FROM indexing_outbox
WHERE note_id = $1",
		)
		.bind(note_id)
		.fetch_optional(pool)
		.await
		.ok()
		.flatten();

		if let Some(row) = row
			&& row.status == status
		{
			return Some(row);
		}

		if Instant::now() >= deadline {
			return None;
		}

		tokio::time::sleep(Duration::from_millis(200)).await;
	}
}

async fn start_embed_server(request_count: Arc<AtomicUsize>) -> (String, Sender<()>) {
	let app =
		Router::new().route("/embeddings", routing::post(embed_handler)).with_state(request_count);
	let listener = TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind embed server.");
	let addr = listener.local_addr().expect("Failed to read embed server address.");
	let (tx, rx) = oneshot::channel();
	let server = axum::serve(listener, app).with_graceful_shutdown(async move {
		let _ = rx.await;
	});

	tokio::spawn(async move {
		let _ = server.into_future().await;
	});

	(format!("http://{addr}"), tx)
}

async fn embed_handler(
	State(counter): State<Arc<AtomicUsize>>,
	Json(payload): Json<Value>,
) -> impl IntoResponse {
	let call_index = counter.fetch_add(1, Ordering::SeqCst);

	if call_index == 0 {
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	}

	let inputs =
		payload.get("input").and_then(|value| value.as_array()).cloned().unwrap_or_default();
	let data: Vec<_> = inputs
		.iter()
		.enumerate()
		.map(|(index, _)| {
			let embedding: Vec<f32> = vec![0.1_f32; 4_096];

			serde_json::json!({
				"index": index,
				"embedding": embedding
			})
		})
		.collect();

	(StatusCode::OK, Json(serde_json::json!({ "data": data }))).into_response()
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn outbox_retries_to_done() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping outbox_retries_to_done; set ELF_PG_DSN to run this test.");

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!("Skipping outbox_retries_to_done; set ELF_QDRANT_URL to run this test.");

		return;
	};
	let request_count = Arc::new(AtomicUsize::new(0));
	let (api_base, shutdown) = start_embed_server(request_count.clone()).await;
	let extractor = SpyExtractor {
		calls: Arc::new(AtomicUsize::new(0)),
		payload: serde_json::json!({ "notes": [] }),
	};
	let providers = Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(StubRerank),
		Arc::new(extractor),
	);
	let collection = test_db.collection_name("elf_acceptance");
	let cfg =
		crate::acceptance::test_config(test_db.dsn().to_string(), qdrant_url, 4_096, collection);
	let service =
		crate::acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	crate::acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");
	crate::acceptance::reset_qdrant_collection(
		&service.qdrant.client,
		&service.qdrant.collection,
		service.qdrant.vector_dim,
	)
	.await
	.expect("Failed to reset Qdrant collection.");

	let add_response = service
		.add_note(AddNoteRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "agent_private".to_string(),
			notes: vec![AddNoteInput {
				r#type: "fact".to_string(),
				key: Some("outbox_test".to_string()),
				text: "Fact: Outbox should retry.".to_string(),
				structured: None,
				importance: 0.4,
				confidence: 0.9,
				ttl_days: None,
				source_ref: serde_json::json!({}),
			}],
		})
		.await
		.expect("Failed to add note.");
	let note_id = add_response.results[0].note_id.expect("Expected note_id in add_note result.");
	let worker_state = worker::WorkerState {
		db: Db::connect(&service.cfg.storage.postgres).await.expect("Failed to connect worker DB."),
		qdrant: QdrantStore::new(&service.cfg.storage.qdrant)
			.expect("Failed to build Qdrant store."),
		embedding: EmbeddingProviderConfig {
			provider_id: "test".to_string(),
			api_base,
			api_key: "test-key".to_string(),
			path: "/embeddings".to_string(),
			model: "test".to_string(),
			dimensions: 4_096,
			timeout_ms: 1_000,
			default_headers: Map::new(),
		},
		chunking: crate::acceptance::chunking::ChunkingConfig { max_tokens: 64, overlap_tokens: 8 },
		tokenizer: {
			let mut vocab = AHashMap::new();

			vocab.insert("<unk>".to_string(), 0_u32);

			let model = WordLevel::builder()
				.vocab(vocab)
				.unk_token("<unk>".to_string())
				.build()
				.expect("Failed to build test tokenizer.");

			Tokenizer::new(model)
		},
	};
	let handle = tokio::spawn(async move {
		let _ = worker::run_worker(worker_state).await;
	});
	let failed = wait_for_status(&service.db.pool, note_id, "FAILED", Duration::from_secs(5))
		.await
		.expect("Expected FAILED outbox status.");

	assert_eq!(failed.attempts, 1);
	assert!(failed.last_error.is_some());
	assert!(request_count.load(Ordering::SeqCst) >= 1);

	let now = OffsetDateTime::now_utc();

	sqlx::query("UPDATE indexing_outbox SET available_at = $1 WHERE note_id = $2")
		.bind(now)
		.bind(note_id)
		.execute(&service.db.pool)
		.await
		.expect("Failed to update available_at.");

	let done = wait_for_status(&service.db.pool, note_id, "DONE", Duration::from_secs(5))
		.await
		.expect("Expected DONE outbox status.");

	assert!(done.attempts >= 1);

	handle.abort();

	let _ = shutdown.send(());

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
