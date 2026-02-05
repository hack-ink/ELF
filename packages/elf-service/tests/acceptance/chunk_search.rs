use qdrant_client::{
	client::Payload,
	qdrant::{CreateCollectionBuilder, Distance, Document, Modifier, PointStruct, SparseVectorParamsBuilder, SparseVectorsConfigBuilder, UpsertPointsBuilder, Vector, VectorParamsBuilder, VectorsConfigBuilder},
};

use super::{SpyExtractor, StubEmbedding, StubRerank, build_service, test_config, test_db, test_qdrant_url};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn search_returns_chunk_items() {
	let Some(test_db) = test_db().await else {
		eprintln!("Skipping search_returns_chunk_items; set ELF_PG_DSN to run this test.");
		return;
	};
	let Some(qdrant_url) = test_qdrant_url() else {
		eprintln!("Skipping search_returns_chunk_items; set ELF_QDRANT_URL to run this test.");
		return;
	};

	let collection = test_db.collection_name("elf_acceptance");
	let cfg = test_config(test_db.dsn().to_string(), qdrant_url, 3, collection);
	let providers = elf_service::Providers::new(
		std::sync::Arc::new(StubEmbedding { vector_dim: 3 }),
		std::sync::Arc::new(StubRerank),
		std::sync::Arc::new(SpyExtractor {
			calls: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
			payload: serde_json::json!({ "notes": [] }),
		}),
	);
	let service = build_service(cfg, providers).await.expect("Failed to build service.");
	super::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

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

	let note_id = uuid::Uuid::new_v4();
	let chunk_id = uuid::Uuid::new_v4();
	let now = time::OffsetDateTime::now_utc();
	let embedding_version = format!(
		"{}:{}:{}",
		service.cfg.providers.embedding.provider_id,
		service.cfg.providers.embedding.model,
		service.cfg.storage.qdrant.vector_dim
	);
	let note_text = "First sentence. Second sentence.";

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
	.bind(&embedding_version)
	.bind(serde_json::json!({}))
	.bind(0_i64)
	.bind::<Option<time::OffsetDateTime>>(None)
	.execute(&service.db.pool)
	.await
	.expect("Failed to insert memory note.");

	sqlx::query(
		"INSERT INTO memory_note_chunks \
         (chunk_id, note_id, chunk_index, start_offset, end_offset, text, embedding_version) \
         VALUES ($1,$2,$3,$4,$5,$6,$7)",
	)
	.bind(chunk_id)
	.bind(note_id)
	.bind(0_i32)
	.bind(0_i32)
	.bind(note_text.len() as i32)
	.bind(note_text)
	.bind(&embedding_version)
	.execute(&service.db.pool)
	.await
	.expect("Failed to insert chunk metadata.");

	let mut payload = Payload::new();
	payload.insert("note_id", note_id.to_string());
	payload.insert("chunk_id", chunk_id.to_string());
	payload.insert("chunk_index", serde_json::Value::from(0));
	payload.insert("start_offset", serde_json::Value::from(0));
	payload.insert("end_offset", serde_json::Value::from(note_text.len() as i32));
	payload.insert("tenant_id", "t");
	payload.insert("project_id", "p");
	payload.insert("agent_id", "a");
	payload.insert("scope", "agent_private");
	payload.insert("status", "active");

	let mut vectors = std::collections::HashMap::new();
	vectors.insert(elf_storage::qdrant::DENSE_VECTOR_NAME.to_string(), Vector::from(vec![0.0; 3]));
	vectors.insert(
		elf_storage::qdrant::BM25_VECTOR_NAME.to_string(),
		Vector::from(Document::new(note_text.to_string(), elf_storage::qdrant::BM25_MODEL)),
	);
	let point = PointStruct::new(chunk_id.to_string(), vectors, payload);
	service
		.qdrant
		.client
		.upsert_points(
			UpsertPointsBuilder::new(service.qdrant.collection.clone(), vec![point]).wait(true),
		)
		.await
		.expect("Failed to upsert Qdrant point.");

	let response = service
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

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
