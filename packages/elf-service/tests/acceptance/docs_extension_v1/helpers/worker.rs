use std::future::IntoFuture;

use ahash::AHashMap;
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing};
use serde_json::{Map, Value};
use tokenizers::{Tokenizer, models::wordlevel::WordLevel};
use tokio::{
	net::TcpListener,
	sync::{oneshot, oneshot::Sender},
	task::JoinHandle,
};

use crate::acceptance::chunking::ChunkingConfig;
use elf_config::EmbeddingProviderConfig;
use elf_service::ElfService;
use elf_storage::{db::Db, qdrant::QdrantStore};
use elf_worker::worker::{self, WorkerState};

pub(crate) async fn spawn_doc_worker(service: &ElfService) -> (JoinHandle<()>, Sender<()>) {
	let (api_base, shutdown) = start_embed_server().await;
	let worker_state = WorkerState {
		db: Db::connect(&service.cfg.storage.postgres).await.expect("Failed to connect worker DB."),
		qdrant: QdrantStore::new(&service.cfg.storage.qdrant)
			.expect("Failed to build Qdrant store."),
		docs_qdrant: QdrantStore::new_with_collection(
			&service.cfg.storage.qdrant,
			&service.cfg.storage.qdrant.docs_collection,
		)
		.expect("Failed to build docs Qdrant store."),
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
		chunking: ChunkingConfig { max_tokens: 64, overlap_tokens: 8 },
		tokenizer: build_test_tokenizer(),
	};
	let handle = tokio::spawn(async move {
		let _ = worker::run_worker(worker_state).await;
	});

	(handle, shutdown)
}

fn build_test_tokenizer() -> Tokenizer {
	let mut vocab = AHashMap::new();

	vocab.insert("<unk>".to_string(), 0_u32);

	let model = WordLevel::builder()
		.vocab(vocab)
		.unk_token("<unk>".to_string())
		.build()
		.expect("Failed to build test tokenizer.");

	Tokenizer::new(model)
}

async fn start_embed_server() -> (String, Sender<()>) {
	let app = Router::new().route("/embeddings", routing::post(embed_handler)).with_state(());
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

async fn embed_handler(State(()): State<()>, Json(payload): Json<Value>) -> impl IntoResponse {
	let inputs =
		payload.get("input").and_then(|value| value.as_array()).cloned().unwrap_or_default();
	let data: Vec<_> = inputs
		.iter()
		.enumerate()
		.map(|(index, _)| {
			let embedding: Vec<f32> = vec![0.1_f32; 4_096];

			serde_json::json!({
				"index": index,
				"embedding": embedding,
			})
		})
		.collect();

	(StatusCode::OK, Json(serde_json::json!({ "data": data }))).into_response()
}
