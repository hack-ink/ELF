use qdrant_client::{
	client::Payload,
	qdrant::{
		CreateCollectionBuilder, Distance, Document, Modifier, PointStruct,
		SparseVectorParamsBuilder, SparseVectorsConfigBuilder, UpsertPointsBuilder, Vector,
		VectorParamsBuilder, VectorsConfigBuilder,
	},
};

use super::{
	SpyExtractor, StubEmbedding, StubRerank, build_service, test_config, test_db, test_qdrant_url,
};

struct TestContext {
	service: elf_service::ElfService,
	test_db: elf_testkit::TestDatabase,
	embedding_version: String,
}

struct KeywordRerank {
	keyword: &'static str,
}

impl elf_service::RerankProvider for KeywordRerank {
	fn rerank<'a>(
		&'a self,
		_cfg: &'a elf_config::ProviderConfig,
		_query: &'a str,
		docs: &'a [String],
	) -> elf_service::BoxFuture<'a, color_eyre::Result<Vec<f32>>> {
		let keyword = self.keyword;
		Box::pin(async move {
			Ok(docs.iter().map(|doc| if doc.contains(keyword) { 1.0 } else { 0.1 }).collect())
		})
	}
}

fn build_providers<R>(rerank: R) -> elf_service::Providers
where
	R: elf_service::RerankProvider + Send + Sync + 'static,
{
	elf_service::Providers::new(
		std::sync::Arc::new(StubEmbedding { vector_dim: 3 }),
		std::sync::Arc::new(rerank),
		std::sync::Arc::new(SpyExtractor {
			calls: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
			payload: serde_json::json!({ "notes": [] }),
		}),
	)
}

async fn setup_context(test_name: &str, providers: elf_service::Providers) -> Option<TestContext> {
	let Some(test_db) = test_db().await else {
		eprintln!("Skipping {test_name}; set ELF_PG_DSN to run this test.");
		return None;
	};
	let Some(qdrant_url) = test_qdrant_url() else {
		eprintln!("Skipping {test_name}; set ELF_QDRANT_URL to run this test.");
		return None;
	};

	let collection = test_db.collection_name("elf_acceptance");
	let cfg = test_config(test_db.dsn().to_string(), qdrant_url, 3, collection);
	let service = build_service(cfg, providers).await.expect("Failed to build service.");
	super::reset_db(&service.db.pool).await.expect("Failed to reset test database.");
	reset_collection(&service).await;

	let embedding_version = format!(
		"{}:{}:{}",
		service.cfg.providers.embedding.provider_id,
		service.cfg.providers.embedding.model,
		service.cfg.storage.qdrant.vector_dim
	);
	Some(TestContext { service, test_db, embedding_version })
}

async fn reset_collection(service: &elf_service::ElfService) {
	let _ = service.qdrant.client.delete_collection(service.qdrant.collection.clone()).await;
	let mut vectors_config = VectorsConfigBuilder::default();
	vectors_config.add_named_vector_params(
		elf_storage::qdrant::DENSE_VECTOR_NAME,
		VectorParamsBuilder::new(3, Distance::Cosine),
	);
	let mut sparse_vectors_config = SparseVectorsConfigBuilder::default();
	sparse_vectors_config.add_named_vector_params(
		elf_storage::qdrant::BM25_VECTOR_NAME,
		SparseVectorParamsBuilder::default().modifier(Modifier::Idf as i32),
	);
	service
		.qdrant
		.client
		.create_collection(
			CreateCollectionBuilder::new(service.qdrant.collection.clone())
				.vectors_config(vectors_config)
				.sparse_vectors_config(sparse_vectors_config),
		)
		.await
		.expect("Failed to create Qdrant collection.");
}

async fn insert_note(
	pool: &sqlx::PgPool,
	note_id: uuid::Uuid,
	note_text: &str,
	embedding_version: &str,
) {
	let now = time::OffsetDateTime::now_utc();
	sqlx::query(
		"INSERT INTO memory_notes \
         (note_id, tenant_id, project_id, agent_id, scope, type, key, text, importance, confidence, status, created_at, updated_at, expires_at, embedding_version, source_ref, hit_count, last_hit_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)",
	)
	.bind(note_id)
	.bind("t")
	.bind("p")
	.bind("a")
	.bind("agent_private")
	.bind("fact")
	.bind::<Option<String>>(None)
	.bind(note_text)
	.bind(0.4_f32)
	.bind(0.9_f32)
	.bind("active")
	.bind(now)
	.bind(now)
	.bind::<Option<time::OffsetDateTime>>(None)
	.bind(embedding_version)
	.bind(serde_json::json!({}))
	.bind(0_i64)
	.bind::<Option<time::OffsetDateTime>>(None)
	.execute(pool)
	.await
	.expect("Failed to insert memory note.");
}

#[allow(clippy::too_many_arguments)]
async fn insert_chunk(
	pool: &sqlx::PgPool,
	chunk_id: uuid::Uuid,
	note_id: uuid::Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	text: &str,
	embedding_version: &str,
) {
	sqlx::query(
		"INSERT INTO memory_note_chunks \
         (chunk_id, note_id, chunk_index, start_offset, end_offset, text, embedding_version) \
         VALUES ($1,$2,$3,$4,$5,$6,$7)",
	)
	.bind(chunk_id)
	.bind(note_id)
	.bind(chunk_index)
	.bind(start_offset)
	.bind(end_offset)
	.bind(text)
	.bind(embedding_version)
	.execute(pool)
	.await
	.expect("Failed to insert chunk metadata.");
}

fn build_payload(
	note_id: uuid::Uuid,
	chunk_id: uuid::Uuid,
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

fn build_vectors(text: &str) -> std::collections::HashMap<String, Vector> {
	let mut vectors = std::collections::HashMap::new();
	vectors.insert(elf_storage::qdrant::DENSE_VECTOR_NAME.to_string(), Vector::from(vec![0.0; 3]));
	vectors.insert(
		elf_storage::qdrant::BM25_VECTOR_NAME.to_string(),
		Vector::from(Document::new(text.to_string(), elf_storage::qdrant::BM25_MODEL)),
	);
	vectors
}

async fn upsert_point(
	service: &elf_service::ElfService,
	chunk_id: uuid::Uuid,
	note_id: uuid::Uuid,
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

	let note_id = uuid::Uuid::new_v4();
	let chunk_id = uuid::Uuid::new_v4();
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
		.search(elf_service::SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			read_profile: "private_only".to_string(),
			query: "First".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			record_hits: Some(false),
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

	let note_id = uuid::Uuid::new_v4();
	let chunk_texts = ["First sentence. ", "Second sentence. ", "Third sentence."];
	let note_text = chunk_texts.concat();
	insert_note(&context.service.db.pool, note_id, &note_text, &context.embedding_version).await;

	let mut offset = 0_i32;
	let mut chunk_ids = Vec::new();
	for (index, chunk_text) in chunk_texts.iter().enumerate() {
		let chunk_id = uuid::Uuid::new_v4();
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
		.search(elf_service::SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			read_profile: "private_only".to_string(),
			query: "Second".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			record_hits: Some(false),
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

	let note_id = uuid::Uuid::new_v4();
	let chunk_id = uuid::Uuid::new_v4();
	let note_text = "Missing chunk metadata.";
	insert_note(&context.service.db.pool, note_id, note_text, &context.embedding_version).await;

	upsert_point(&context.service, chunk_id, note_id, 0, 0, note_text.len() as i32, note_text)
		.await;

	let response = context
		.service
		.search(elf_service::SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			read_profile: "private_only".to_string(),
			query: "Missing".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			record_hits: Some(false),
		})
		.await
		.expect("Search failed.");

	assert!(response.items.is_empty());

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn search_dedupes_note_results() {
	let providers = build_providers(KeywordRerank { keyword: "preferred" });
	let Some(context) = setup_context("search_dedupes_note_results", providers).await else {
		return;
	};

	let note_id = uuid::Uuid::new_v4();
	let chunk_texts = ["preferred alpha. ", "bridge chunk. ", "other alpha."];
	let note_text = chunk_texts.concat();
	insert_note(&context.service.db.pool, note_id, &note_text, &context.embedding_version).await;

	let mut offset = 0_i32;
	let mut chunk_ids = Vec::new();
	for (index, chunk_text) in chunk_texts.iter().enumerate() {
		let chunk_id = uuid::Uuid::new_v4();
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
		.search(elf_service::SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			read_profile: "private_only".to_string(),
			query: "alpha".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			record_hits: Some(false),
		})
		.await
		.expect("Search failed.");

	let item = response.items.first().expect("Expected search result.");
	assert_eq!(response.items.len(), 1);
	assert_eq!(item.chunk_id, chunk_id_a);

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}
