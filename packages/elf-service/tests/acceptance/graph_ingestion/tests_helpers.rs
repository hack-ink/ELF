use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
	sync::{Arc, atomic::AtomicUsize},
};

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::acceptance::{self, SpyExtractor, StubEmbedding, StubRerank};
use elf_config::EmbeddingProviderConfig;
use elf_domain::memory_policy::MemoryPolicyDecision;
use elf_service::{
	AddNoteInput, AddNoteRequest, BoxFuture, ElfService, EmbeddingProvider, NoteOp, Providers,
	Result, StructuredFields,
};
use elf_testkit::TestDatabase;

pub(super) const TEST_TENANT: &str = "t";
pub(super) const TEST_PROJECT: &str = "p";
pub(super) const TEST_SCOPE: &str = "agent_private";
pub(super) const GRAPH_REL_SUBJECT: &str = "alice";
pub(super) const GRAPH_REL_PREDICATE: &str = "mentors";
pub(super) const GRAPH_REL_OBJECT: &str = "Bob";

pub(super) struct HashEmbedding {
	vector_dim: u32,
}
impl EmbeddingProvider for HashEmbedding {
	fn embed<'a>(
		&'a self,
		_: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, Result<Vec<Vec<f32>>>> {
		let vector_dim = self.vector_dim as usize;
		let vectors = texts
			.iter()
			.map(|text| {
				let mut values = Vec::with_capacity(vector_dim);

				for idx in 0..vector_dim {
					let mut hasher = DefaultHasher::new();

					text.hash(&mut hasher);
					idx.hash(&mut hasher);

					let raw = hasher.finish();
					let normalized = ((raw % 2_000_000) as f32 / 1_000_000.0) - 1.0;

					values.push(normalized);
				}

				values
			})
			.collect();

		Box::pin(async move { Ok(vectors) })
	}
}

pub(super) fn fact_note(
	key: &str,
	text: &str,
	predicate: &str,
	object_value: &str,
) -> AddNoteInput {
	let structured = serde_json::from_value::<StructuredFields>(serde_json::json!({
		"relations": [{
			"subject": { "canonical": "Alice" },
			"predicate": predicate,
			"object": { "value": object_value }
		}]
	}))
	.expect("Failed to build structured fields.");

	AddNoteInput {
		r#type: "fact".to_string(),
		key: Some(key.to_string()),
		text: text.to_string(),
		structured: Some(structured),
		importance: 0.8,
		confidence: 0.9,
		ttl_days: None,
		source_ref: serde_json::json!({}),
		write_policy: None,
	}
}

pub(super) fn assert_graph_policy_from_op(op: NoteOp, policy_decision: MemoryPolicyDecision) {
	match op {
		NoteOp::Add => assert_eq!(policy_decision, MemoryPolicyDecision::Remember),
		NoteOp::Update => assert_eq!(policy_decision, MemoryPolicyDecision::Update),
		_ => {},
	}
}

pub(super) fn duplicate_fact_attaches_multiple_evidence_request() -> AddNoteRequest {
	AddNoteRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "agent_private".to_string(),
		notes: vec![
			AddNoteInput {
				r#type: "fact".to_string(),
				key: Some("mentorship-a".to_string()),
				text: "Alice mentors Bob in 2026.".to_string(),
				structured: Some(
					serde_json::from_value::<elf_service::structured_fields::StructuredFields>(
						serde_json::json!({
							"relations": [{
								"subject": { "canonical": "Alice" },
								"predicate": "mentors",
								"object": { "value": "Bob" }
							}]
						}),
					)
					.expect("Failed to build structured fields."),
				),
				importance: 0.8,
				confidence: 0.9,
				ttl_days: None,
				source_ref: serde_json::json!({}),
				write_policy: None,
			},
			AddNoteInput {
				r#type: "fact".to_string(),
				key: Some("mentorship-b".to_string()),
				text: "Alice also mentors Bob often.".to_string(),
				structured: Some(
					serde_json::from_value::<elf_service::structured_fields::StructuredFields>(
						serde_json::json!({
							"relations": [{
								"subject": { "canonical": "Alice" },
								"predicate": "mentors",
								"object": { "value": "Bob" }
							}]
						}),
					)
					.expect("Failed to build structured fields."),
				),
				importance: 0.7,
				confidence: 0.8,
				ttl_days: None,
				source_ref: serde_json::json!({}),
				write_policy: None,
			},
		],
	}
}

pub(super) async fn graph_fact_id(pool: &PgPool) -> Uuid {
	sqlx::query_scalar(
		"\
SELECT gf.fact_id
FROM graph_facts gf
JOIN graph_entities ge ON ge.entity_id = gf.subject_entity_id
WHERE ge.canonical_norm = $1
	AND gf.predicate = $2
	AND gf.object_value = $3
	AND gf.tenant_id = $4
	AND gf.project_id = $5
	AND gf.scope = $6",
	)
	.bind(GRAPH_REL_SUBJECT)
	.bind(GRAPH_REL_PREDICATE)
	.bind(GRAPH_REL_OBJECT)
	.bind(TEST_TENANT)
	.bind(TEST_PROJECT)
	.bind(TEST_SCOPE)
	.fetch_one(pool)
	.await
	.expect("Failed to load fact.")
}

pub(super) async fn graph_fact_count(pool: &PgPool) -> i64 {
	sqlx::query_scalar(
		"\
SELECT COUNT(*)
FROM graph_facts gf
JOIN graph_entities ge ON ge.entity_id = gf.subject_entity_id
WHERE ge.canonical_norm = $1
	AND gf.predicate = $2
	AND gf.object_value = $3
	AND gf.tenant_id = $4
	AND gf.project_id = $5
	AND gf.scope = $6",
	)
	.bind(GRAPH_REL_SUBJECT)
	.bind(GRAPH_REL_PREDICATE)
	.bind(GRAPH_REL_OBJECT)
	.bind(TEST_TENANT)
	.bind(TEST_PROJECT)
	.bind(TEST_SCOPE)
	.fetch_one(pool)
	.await
	.expect("Failed to count fact rows.")
}

pub(super) async fn graph_fact_evidence_count(pool: &PgPool, fact_id: Uuid) -> i64 {
	sqlx::query_scalar("SELECT COUNT(*) FROM graph_fact_evidence WHERE fact_id = $1")
		.bind(fact_id)
		.fetch_one(pool)
		.await
		.expect("Failed to load fact evidence.")
}

pub(super) async fn graph_fact_evidence_count_for_note(
	pool: &PgPool,
	fact_id: Uuid,
	note_id: Uuid,
) -> i64 {
	sqlx::query_scalar(
		"SELECT COUNT(*) FROM graph_fact_evidence WHERE fact_id = $1 AND note_id = $2",
	)
	.bind(fact_id)
	.bind(note_id)
	.fetch_one(pool)
	.await
	.expect("Failed to load note evidence.")
}

pub(super) async fn add_fact_note(
	service: &ElfService,
	key: &str,
	text: &str,
	predicate: &str,
	object_value: &str,
) -> Uuid {
	let response = service
		.add_note(AddNoteRequest {
			tenant_id: TEST_TENANT.to_string(),
			project_id: TEST_PROJECT.to_string(),
			agent_id: "a".to_string(),
			scope: TEST_SCOPE.to_string(),
			notes: vec![fact_note(key, text, predicate, object_value)],
		})
		.await
		.expect("add_note failed.");

	assert_eq!(response.results.len(), 1);
	assert_eq!(response.results[0].op, NoteOp::Add);

	assert_graph_policy_from_op(response.results[0].op, response.results[0].policy_decision);

	response.results[0].note_id.expect("Expected note_id.")
}

pub(super) async fn build_test_db(test_name: &str) -> Option<TestDatabase> {
	let Some(test_db) = acceptance::test_db().await else {
		eprintln!("Skipping {test_name}; set ELF_PG_DSN to run.");

		return None;
	};
	let Some(_) = acceptance::test_qdrant_url() else {
		eprintln!("Skipping {test_name}; set ELF_QDRANT_URL to run.");

		return None;
	};

	Some(test_db)
}

pub(super) async fn build_hash_service(test_db: &TestDatabase) -> ElfService {
	let qdrant_url = acceptance::test_qdrant_url().expect("Expected Qdrant test URL.");
	let providers = Providers::new(
		Arc::new(HashEmbedding { vector_dim: 4_096 }),
		Arc::new(StubRerank),
		Arc::new(SpyExtractor {
			calls: Arc::new(AtomicUsize::new(0)),
			payload: serde_json::json!({ "notes": [] }),
		}),
	);
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let cfg = acceptance::test_config(
		test_db.dsn().to_string(),
		qdrant_url,
		4_096,
		collection,
		docs_collection,
	);

	acceptance::build_service(cfg, providers).await.expect("Failed to build service.")
}

pub(super) async fn build_stub_service(test_db: &TestDatabase) -> ElfService {
	build_service_with_extractor_payload(test_db, serde_json::json!({ "notes": [] })).await
}

pub(super) async fn build_service_with_extractor_payload(
	test_db: &TestDatabase,
	extractor_payload: Value,
) -> ElfService {
	let qdrant_url = acceptance::test_qdrant_url().expect("Expected Qdrant test URL.");
	let providers = Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(StubRerank),
		Arc::new(SpyExtractor { calls: Arc::new(AtomicUsize::new(0)), payload: extractor_payload }),
	);
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let cfg = acceptance::test_config(
		test_db.dsn().to_string(),
		qdrant_url,
		4_096,
		collection,
		docs_collection,
	);

	acceptance::build_service(cfg, providers).await.expect("Failed to build service.")
}

pub(super) async fn reset_service_db(service: &ElfService) {
	acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");
}
