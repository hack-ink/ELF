use std::collections::HashMap;

use qdrant_client::{
	client::Payload,
	qdrant::{Document, PointStruct, UpsertPointsBuilder, Vector},
};
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use elf_config::ProviderConfig;
use elf_service::{BoxFuture, ElfService, Providers, RerankProvider, Result, SearchRequest};
use elf_storage::qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME};
use elf_testkit::TestDatabase;

struct TestContext {
	service: ElfService,
	test_db: TestDatabase,
	embedding_version: String,
}

struct UpsertPointArgs<'a> {
	chunk_id: Uuid,
	note_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	text: &'a str,
	dense: Vec<f32>,
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

fn vec_text_zeros() -> String {
	let mut buf = String::with_capacity(2 + (4_096 * 2));

	buf.push('[');

	for i in 0..4_096 {
		if i > 0 {
			buf.push(',');
		}

		buf.push('0');
	}

	buf.push(']');

	buf
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
	payload.insert("chunk_index", serde_json::Value::from(chunk_index));
	payload.insert("start_offset", serde_json::Value::from(start_offset));
	payload.insert("end_offset", serde_json::Value::from(end_offset));
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

async fn setup_context(test_name: &str) -> Option<TestContext> {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping {test_name}; set ELF_PG_DSN to run this test.");

		return None;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!("Skipping {test_name}; set ELF_QDRANT_URL to run this test.");

		return None;
	};
	let providers = Providers::new(
		std::sync::Arc::new(crate::acceptance::StubEmbedding { vector_dim: 4_096 }),
		std::sync::Arc::new(KeywordRerank { keyword: "ZEBRA" }),
		std::sync::Arc::new(crate::acceptance::SpyExtractor {
			calls: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
			payload: serde_json::json!({ "notes": [] }),
		}),
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

	let embedding_version = format!(
		"{}:{}:{}",
		service.cfg.providers.embedding.provider_id,
		service.cfg.providers.embedding.model,
		service.cfg.storage.qdrant.vector_dim
	);

	Some(TestContext { service, test_db, embedding_version })
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

async fn insert_chunk_embedding<'e, E>(executor: E, chunk_id: Uuid, embedding_version: &str)
where
	E: PgExecutor<'e>,
{
	let vec_text = vec_text_zeros();

	sqlx::query(
		"\
INSERT INTO note_chunk_embeddings (chunk_id, embedding_version, embedding_dim, vec)
VALUES ($1, $2, $3, $4::text::vector)",
	)
	.bind(chunk_id)
	.bind(embedding_version)
	.bind(4_096_i32)
	.bind(vec_text.as_str())
	.execute(executor)
	.await
	.expect("Failed to insert chunk embedding.");
}

async fn insert_fact_field_row<'e, E>(executor: E, field_id: Uuid, note_id: Uuid, fact_text: &str)
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO memory_note_fields (field_id, note_id, field_kind, item_index, text)
VALUES ($1, $2, $3, $4, $5)",
	)
	.bind(field_id)
	.bind(note_id)
	.bind("fact")
	.bind(0_i32)
	.bind(fact_text)
	.execute(executor)
	.await
	.expect("Failed to insert note field.");
}

async fn insert_fact_field_embedding<'e, E>(executor: E, field_id: Uuid, embedding_version: &str)
where
	E: PgExecutor<'e>,
{
	let vec_text = vec_text_zeros();

	sqlx::query(
		"\
INSERT INTO note_field_embeddings (field_id, embedding_version, embedding_dim, vec)
VALUES ($1, $2, $3, $4::text::vector)",
	)
	.bind(field_id)
	.bind(embedding_version)
	.bind(4_096_i32)
	.bind(vec_text.as_str())
	.execute(executor)
	.await
	.expect("Failed to insert field embedding.");
}

async fn upsert_point(service: &ElfService, args: UpsertPointArgs<'_>) {
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

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn structured_fact_field_can_surface_note_and_marks_matched_fields() {
	let Some(context) =
		setup_context("structured_fact_field_can_surface_note_and_marks_matched_fields").await
	else {
		return;
	};
	let query = "alpha unique";

	for i in 0..20 {
		let note_id = Uuid::new_v4();
		let chunk_id = Uuid::new_v4();
		let text = format!("Confuser {i}: {query}.");

		insert_note(&context.service.db.pool, note_id, &text, &context.embedding_version).await;
		insert_chunk(
			&context.service.db.pool,
			chunk_id,
			note_id,
			0,
			0,
			text.len() as i32,
			&text,
			&context.embedding_version,
		)
		.await;
		upsert_point(
			&context.service,
			UpsertPointArgs {
				chunk_id,
				note_id,
				chunk_index: 0,
				start_offset: 0,
				end_offset: text.len() as i32,
				text: &text,
				dense: vec![0.0_f32; 4_096],
			},
		)
		.await;
	}

	let structured_note_id = Uuid::new_v4();
	let structured_chunk_id = Uuid::new_v4();
	let structured_chunk_text = "ZEBRA chunk text does not include the query.";

	insert_note(
		&context.service.db.pool,
		structured_note_id,
		"This note is generic.",
		&context.embedding_version,
	)
	.await;
	insert_chunk(
		&context.service.db.pool,
		structured_chunk_id,
		structured_note_id,
		0,
		0,
		structured_chunk_text.len() as i32,
		structured_chunk_text,
		&context.embedding_version,
	)
	.await;
	insert_chunk_embedding(
		&context.service.db.pool,
		structured_chunk_id,
		&context.embedding_version,
	)
	.await;
	upsert_point(
		&context.service,
		UpsertPointArgs {
			chunk_id: structured_chunk_id,
			note_id: structured_note_id,
			chunk_index: 0,
			start_offset: 0,
			end_offset: structured_chunk_text.len() as i32,
			text: structured_chunk_text,
			dense: vec![1.0_f32; 4_096],
		},
	)
	.await;

	let field_id = Uuid::new_v4();

	insert_fact_field_row(&context.service.db.pool, field_id, structured_note_id, query).await;
	insert_fact_field_embedding(&context.service.db.pool, field_id, &context.embedding_version)
		.await;

	let response = context
		.service
		.search_raw(SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			read_profile: "private_only".to_string(),
			query: query.to_string(),
			top_k: Some(1),
			candidate_k: Some(10),
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search failed.");
	let item = response.items.first().expect("Expected search result.");

	assert_eq!(item.note_id, structured_note_id);
	assert!(
		item.explain.r#match.matched_fields.iter().any(|field| field == "facts"),
		"Expected matched_fields to include facts; got {:?}",
		item.explain.r#match.matched_fields
	);

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}
