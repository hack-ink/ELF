use std::{
	collections::HashMap,
	sync::{Arc, atomic::AtomicUsize},
};

use qdrant_client::{
	client::Payload,
	qdrant::{Document, PointStruct, UpsertPointsBuilder, Vector},
};
use serde_json::Value;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::acceptance::{SpyExtractor, StubEmbedding, StubRerank};
use elf_config::ProviderConfig;
use elf_service::{
	BoxFuture, ElfService, Providers, RerankProvider, Result, SearchDetailsRequest, SearchRequest,
	SearchTimelineRequest,
};
use elf_storage::qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME};
use elf_testkit::TestDatabase;

struct TestContext {
	service: ElfService,
	test_db: TestDatabase,
	embedding_version: String,
}

struct KeywordRerank {
	keyword: &'static str,
}
impl RerankProvider for KeywordRerank {
	fn rerank<'a>(
		&'a self,
		_cfg: &'a ProviderConfig,
		_query: &'a str,
		docs: &'a [String],
	) -> BoxFuture<'a, Result<Vec<f32>>> {
		let keyword = self.keyword;

		Box::pin(async move {
			Ok(docs.iter().map(|doc| if doc.contains(keyword) { 1.0 } else { 0.1 }).collect())
		})
	}
}

fn build_providers<R>(rerank: R) -> Providers
where
	R: RerankProvider + Send + Sync + 'static,
{
	Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(rerank),
		Arc::new(SpyExtractor {
			calls: Arc::new(AtomicUsize::new(0)),
			payload: serde_json::json!({ "notes": [] }),
		}),
	)
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

fn build_vectors(text: &str) -> HashMap<String, Vector> {
	let mut vectors = HashMap::new();

	vectors.insert(DENSE_VECTOR_NAME.to_string(), Vector::from(vec![0.0_f32; 4_096]));
	vectors.insert(
		BM25_VECTOR_NAME.to_string(),
		Vector::from(Document::new(text.to_string(), BM25_MODEL)),
	);

	vectors
}

async fn setup_context(test_name: &str, providers: Providers) -> Option<TestContext> {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping {test_name}; set ELF_PG_DSN to run this test.");

		return None;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!("Skipping {test_name}; set ELF_QDRANT_URL to run this test.");

		return None;
	};
	let collection = test_db.collection_name("elf_acceptance");
	let cfg =
		crate::acceptance::test_config(test_db.dsn().to_string(), qdrant_url, 4_096, collection);
	let service =
		crate::acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	crate::acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

	reset_collection(&service).await;

	let embedding_version = format!(
		"{}:{}:{}",
		service.cfg.providers.embedding.provider_id,
		service.cfg.providers.embedding.model,
		service.cfg.storage.qdrant.vector_dim
	);

	Some(TestContext { service, test_db, embedding_version })
}

async fn reset_collection(service: &ElfService) {
	crate::acceptance::reset_qdrant_collection(
		&service.qdrant.client,
		&service.qdrant.collection,
		service.qdrant.vector_dim,
	)
	.await
	.expect("Failed to reset Qdrant collection.");
}

async fn insert_note<'e, E>(executor: E, note_id: Uuid, note_text: &str, embedding_version: &str)
where
	E: PgExecutor<'e>,
{
	let now = OffsetDateTime::now_utc();

	sqlx::query(
		"\
INSERT INTO memory_notes (
	note_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	type,
	key,
	text,
	importance,
	confidence,
	status,
	created_at,
	updated_at,
	expires_at,
	embedding_version,
	source_ref,
	hit_count,
	last_hit_at
)
VALUES (
	$1,
	$2,
	$3,
	$4,
	$5,
	$6,
	$7,
	$8,
	$9,
	$10,
	$11,
	$12,
	$13,
	$14,
	$15,
	$16,
	$17,
	$18
)",
	)
	.bind(note_id)
	.bind("t")
	.bind("p")
	.bind("a")
	.bind("agent_private")
	.bind("fact")
	.bind(Option::<String>::None)
	.bind(note_text)
	.bind(0.4_f32)
	.bind(0.9_f32)
	.bind("active")
	.bind(now)
	.bind(now)
	.bind(Option::<OffsetDateTime>::None)
	.bind(embedding_version)
	.bind(serde_json::json!({}))
	.bind(0_i64)
	.bind(Option::<OffsetDateTime>::None)
	.execute(executor)
	.await
	.expect("Failed to insert memory note.");
}

#[allow(clippy::too_many_arguments)]
async fn insert_chunk<'e, E>(
	executor: E,
	chunk_id: Uuid,
	note_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	text: &str,
	embedding_version: &str,
) where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO memory_note_chunks (
	chunk_id,
	note_id,
	chunk_index,
	start_offset,
	end_offset,
	text,
	embedding_version
)
VALUES ($1, $2, $3, $4, $5, $6, $7)",
	)
	.bind(chunk_id)
	.bind(note_id)
	.bind(chunk_index)
	.bind(start_offset)
	.bind(end_offset)
	.bind(text)
	.bind(embedding_version)
	.execute(executor)
	.await
	.expect("Failed to insert chunk metadata.");
}

async fn upsert_point(
	service: &ElfService,
	chunk_id: Uuid,
	note_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	text: &str,
) {
	let payload = build_payload(note_id, chunk_id, chunk_index, start_offset, end_offset);
	let vectors = build_vectors(text);
	let point = PointStruct::new(chunk_id.to_string(), vectors, payload);

	service
		.qdrant
		.client
		.upsert_points(
			UpsertPointsBuilder::new(service.qdrant.collection.clone(), vec![point]).wait(true),
		)
		.await
		.expect("Failed to upsert Qdrant point.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn search_returns_chunk_items() {
	let providers = build_providers(StubRerank);
	let Some(context) = setup_context("search_returns_chunk_items", providers).await else {
		return;
	};
	let note_id = Uuid::new_v4();
	let chunk_id = Uuid::new_v4();
	let note_text = "First sentence. Second sentence.";

	insert_note(&context.service.db.pool, note_id, note_text, &context.embedding_version).await;
	insert_chunk(
		&context.service.db.pool,
		chunk_id,
		note_id,
		0,
		0,
		note_text.len() as i32,
		note_text,
		&context.embedding_version,
	)
	.await;
	upsert_point(&context.service, chunk_id, note_id, 0, 0, note_text.len() as i32, note_text)
		.await;

	let response = context
		.service
		.search_raw(SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			read_profile: "private_only".to_string(),
			query: "First".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search failed.");
	let item = response.items.first().expect("Expected search result.");

	assert_eq!(item.chunk_id, chunk_id);
	assert!(!item.snippet.is_empty());

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn search_stitches_adjacent_chunks() {
	let providers = build_providers(StubRerank);
	let Some(context) = setup_context("search_stitches_adjacent_chunks", providers).await else {
		return;
	};
	let note_id = Uuid::new_v4();
	let chunk_texts = ["First sentence. ", "Second sentence. ", "Third sentence."];
	let note_text = chunk_texts.concat();

	insert_note(&context.service.db.pool, note_id, &note_text, &context.embedding_version).await;

	let mut offset = 0_i32;
	let mut chunk_ids = Vec::new();

	for (index, chunk_text) in chunk_texts.iter().enumerate() {
		let chunk_id = Uuid::new_v4();
		let start = offset;
		let end = start + chunk_text.len() as i32;

		insert_chunk(
			&context.service.db.pool,
			chunk_id,
			note_id,
			index as i32,
			start,
			end,
			chunk_text,
			&context.embedding_version,
		)
		.await;

		chunk_ids.push((chunk_id, start, end, *chunk_text));

		offset = end;
	}

	let (chunk_id, start, end, text) = chunk_ids[1];

	upsert_point(&context.service, chunk_id, note_id, 1, start, end, text).await;

	let response = context
		.service
		.search_raw(SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			read_profile: "private_only".to_string(),
			query: "Second".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search failed.");
	let item = response.items.first().expect("Expected search result.");

	assert_eq!(item.chunk_id, chunk_id);
	assert!(item.snippet.contains("First sentence."));
	assert!(item.snippet.contains("Second sentence."));
	assert!(item.snippet.contains("Third sentence."));

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn search_skips_missing_chunk_metadata() {
	let providers = build_providers(StubRerank);
	let Some(context) = setup_context("search_skips_missing_chunk_metadata", providers).await
	else {
		return;
	};
	let note_id = Uuid::new_v4();
	let chunk_id = Uuid::new_v4();
	let note_text = "Missing chunk metadata.";

	insert_note(&context.service.db.pool, note_id, note_text, &context.embedding_version).await;
	upsert_point(&context.service, chunk_id, note_id, 0, 0, note_text.len() as i32, note_text)
		.await;

	let response = context
		.service
		.search_raw(SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			read_profile: "private_only".to_string(),
			query: "Missing".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search failed.");

	assert!(response.items.is_empty());

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn progressive_search_returns_index_timeline_and_details() {
	let providers = build_providers(StubRerank);
	let Some(context) =
		setup_context("progressive_search_returns_index_timeline_and_details", providers).await
	else {
		return;
	};
	let note_id = Uuid::new_v4();
	let chunk_id = Uuid::new_v4();
	let note_text = "Progressive retrieval works best with staged expansion.";

	insert_note(&context.service.db.pool, note_id, note_text, &context.embedding_version).await;
	insert_chunk(
		&context.service.db.pool,
		chunk_id,
		note_id,
		0,
		0,
		note_text.len() as i32,
		note_text,
		&context.embedding_version,
	)
	.await;
	upsert_point(&context.service, chunk_id, note_id, 0, 0, note_text.len() as i32, note_text)
		.await;

	let index = context
		.service
		.search(SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			read_profile: "private_only".to_string(),
			query: "Progressive".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search index failed.");

	assert!(!index.items.is_empty());

	let timeline = context
		.service
		.search_timeline(SearchTimelineRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			search_session_id: index.search_session_id,
			group_by: None,
		})
		.await
		.expect("Search timeline failed.");

	assert!(!timeline.groups.is_empty());

	let details = context
		.service
		.search_details(SearchDetailsRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			search_session_id: index.search_session_id,
			note_ids: vec![note_id],
			record_hits: Some(false),
		})
		.await
		.expect("Search details failed.");
	let returned = details
		.results
		.first()
		.and_then(|result| result.note.as_ref())
		.expect("Expected note details.");

	assert_eq!(returned.note_id, note_id);
	assert_eq!(returned.text, note_text);

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn search_dedupes_note_results() {
	let providers = build_providers(KeywordRerank { keyword: "preferred" });
	let Some(context) = setup_context("search_dedupes_note_results", providers).await else {
		return;
	};
	let note_id = Uuid::new_v4();
	let chunk_texts = ["preferred alpha. ", "bridge chunk. ", "other alpha."];
	let note_text = chunk_texts.concat();

	insert_note(&context.service.db.pool, note_id, &note_text, &context.embedding_version).await;

	let mut offset = 0_i32;
	let mut chunk_ids = Vec::new();

	for (index, chunk_text) in chunk_texts.iter().enumerate() {
		let chunk_id = Uuid::new_v4();
		let start = offset;
		let end = start + chunk_text.len() as i32;

		insert_chunk(
			&context.service.db.pool,
			chunk_id,
			note_id,
			index as i32,
			start,
			end,
			chunk_text,
			&context.embedding_version,
		)
		.await;

		chunk_ids.push((chunk_id, start, end, *chunk_text));

		offset = end;
	}

	let (chunk_id_a, start_a, end_a, text_a) = chunk_ids[0];
	let (chunk_id_c, start_c, end_c, text_c) = chunk_ids[2];

	upsert_point(&context.service, chunk_id_a, note_id, 0, start_a, end_a, text_a).await;
	upsert_point(&context.service, chunk_id_c, note_id, 2, start_c, end_c, text_c).await;

	let response = context
		.service
		.search_raw(SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			read_profile: "private_only".to_string(),
			query: "alpha".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search failed.");
	let item = response.items.first().expect("Expected search result.");

	assert_eq!(response.items.len(), 1);
	assert_eq!(item.note_id, note_id);
	assert!(
		item.chunk_id == chunk_id_a || item.chunk_id == chunk_id_c,
		"Expected deduped result chunk_id to be one of the ingested chunks."
	);

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}
