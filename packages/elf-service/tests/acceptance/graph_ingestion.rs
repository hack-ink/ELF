use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
	sync::{Arc, atomic::AtomicUsize},
};

use sqlx::PgPool;
use uuid::Uuid;

use elf_config::EmbeddingProviderConfig;
use elf_service::{
	AddEventRequest, AddNoteInput, AddNoteRequest, BoxFuture, EmbeddingProvider, EventMessage,
	NoteOp, Providers, Result,
};

const TEST_TENANT: &str = "t";
const TEST_PROJECT: &str = "p";
const TEST_SCOPE: &str = "agent_private";
const GRAPH_REL_SUBJECT: &str = "alice";
const GRAPH_REL_PREDICATE: &str = "mentors";
const GRAPH_REL_OBJECT: &str = "Bob";

struct HashEmbedding {
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

async fn graph_fact_id(pool: &PgPool) -> Uuid {
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

async fn graph_fact_count(pool: &PgPool) -> i64 {
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

async fn graph_fact_evidence_count(pool: &PgPool, fact_id: Uuid) -> i64 {
	sqlx::query_scalar("SELECT COUNT(*) FROM graph_fact_evidence WHERE fact_id = $1")
		.bind(fact_id)
		.fetch_one(pool)
		.await
		.expect("Failed to load fact evidence.")
}

async fn graph_fact_evidence_count_for_note(pool: &PgPool, fact_id: Uuid, note_id: Uuid) -> i64 {
	sqlx::query_scalar(
		"SELECT COUNT(*) FROM graph_fact_evidence WHERE fact_id = $1 AND note_id = $2",
	)
	.bind(fact_id)
	.bind(note_id)
	.fetch_one(pool)
	.await
	.expect("Failed to load note evidence.")
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn add_note_duplicate_fact_attaches_multiple_evidence() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!(
			"Skipping add_note_duplicate_fact_attaches_multiple_evidence; set ELF_PG_DSN to run.",
		);

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!(
			"Skipping add_note_duplicate_fact_attaches_multiple_evidence; set ELF_QDRANT_URL to run.",
		);

		return;
	};
	let providers = Providers::new(
		Arc::new(HashEmbedding { vector_dim: 4_096 }),
		Arc::new(crate::acceptance::StubRerank),
		Arc::new(crate::acceptance::SpyExtractor {
			calls: Arc::new(AtomicUsize::new(0)),
			payload: serde_json::json!({ "notes": [] }),
		}),
	);
	let collection = test_db.collection_name("elf_acceptance");
	let cfg =
		crate::acceptance::test_config(test_db.dsn().to_string(), qdrant_url, 4_096, collection);
	let service =
		crate::acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	crate::acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

	let response = service
		.add_note(AddNoteRequest {
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
				},
			],
		})
		.await
		.expect("add_note failed.");

	assert_eq!(response.results.len(), 2);
	assert_eq!(response.results[0].op, NoteOp::Add);
	assert_eq!(response.results[1].op, NoteOp::Add);

	let first_note_id = response.results[0].note_id.expect("Expected note_id.");
	let second_note_id = response.results[1].note_id.expect("Expected note_id.");

	assert_ne!(first_note_id, second_note_id);

	let fact_id = graph_fact_id(&service.db.pool).await;
	let fact_count = graph_fact_count(&service.db.pool).await;
	let evidence_count = graph_fact_evidence_count(&service.db.pool, fact_id).await;

	assert_eq!(fact_count, 1);
	assert_eq!(evidence_count, 2);

	let first_evidence_count =
		graph_fact_evidence_count_for_note(&service.db.pool, fact_id, first_note_id).await;
	let second_evidence_count =
		graph_fact_evidence_count_for_note(&service.db.pool, fact_id, second_note_id).await;

	assert_eq!(first_evidence_count, 1);
	assert_eq!(second_evidence_count, 1);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn add_note_invalid_relation_rejected_has_field_path() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!(
			"Skipping add_note_invalid_relation_rejected_has_field_path; set ELF_PG_DSN to run."
		);

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!(
			"Skipping add_note_invalid_relation_rejected_has_field_path; set ELF_QDRANT_URL to run.",
		);

		return;
	};
	let providers = Providers::new(
		Arc::new(crate::acceptance::StubEmbedding { vector_dim: 4_096 }),
		Arc::new(crate::acceptance::StubRerank),
		Arc::new(crate::acceptance::SpyExtractor {
			calls: Arc::new(AtomicUsize::new(0)),
			payload: serde_json::json!({ "notes": [] }),
		}),
	);
	let collection = test_db.collection_name("elf_acceptance");
	let cfg =
		crate::acceptance::test_config(test_db.dsn().to_string(), qdrant_url, 4_096, collection);
	let service =
		crate::acceptance::build_service(cfg, providers).await.expect("Failed to build service.");
	let response = service
		.add_note(AddNoteRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "agent_private".to_string(),
			notes: vec![AddNoteInput {
				r#type: "fact".to_string(),
				key: Some("mentorship".to_string()),
				text: "Alice mentors Bob.".to_string(),
				structured: Some(
					serde_json::from_value::<elf_service::structured_fields::StructuredFields>(
						serde_json::json!({
							"relations": [{
								"subject": { "canonical": "Alice" },
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
			}],
		})
		.await
		.expect("add_note failed.");

	assert_eq!(response.results.len(), 1);
	assert_eq!(response.results[0].op, NoteOp::Rejected);
	assert_eq!(response.results[0].reason_code.as_deref(), Some("REJECT_STRUCTURED_INVALID"));
	assert_eq!(
		response.results[0].field_path,
		Some("structured.relations[0].predicate".to_string()),
	);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn add_note_persists_graph_relations() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping add_note_persists_graph_relations; set ELF_PG_DSN to run.");

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!("Skipping add_note_persists_graph_relations; set ELF_QDRANT_URL to run.");

		return;
	};
	let providers = Providers::new(
		Arc::new(crate::acceptance::StubEmbedding { vector_dim: 4_096 }),
		Arc::new(crate::acceptance::StubRerank),
		Arc::new(crate::acceptance::SpyExtractor {
			calls: Arc::new(AtomicUsize::new(0)),
			payload: serde_json::json!({ "notes": [] }),
		}),
	);
	let collection = test_db.collection_name("elf_acceptance");
	let cfg =
		crate::acceptance::test_config(test_db.dsn().to_string(), qdrant_url, 4_096, collection);
	let service =
		crate::acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	crate::acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

	let response = service
		.add_note(AddNoteRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "agent_private".to_string(),
			notes: vec![AddNoteInput {
				r#type: "fact".to_string(),
				key: Some("mentorship".to_string()),
				text: "Alice mentors Bob.".to_string(),
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
			}],
		})
		.await
		.expect("add_note failed.");

	assert_eq!(response.results.len(), 1);
	assert_eq!(response.results[0].op, NoteOp::Add);

	let note_id = response.results[0].note_id.expect("Expected note_id.");
	let fact_id = graph_fact_id(&service.db.pool).await;
	let fact_count = graph_fact_count(&service.db.pool).await;
	let evidence_count =
		graph_fact_evidence_count_for_note(&service.db.pool, fact_id, note_id).await;

	assert_eq!(fact_count, 1);
	assert_eq!(evidence_count, 1);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn add_event_persists_graph_relations() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping add_event_persists_graph_relations; set ELF_PG_DSN to run.");

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!("Skipping add_event_persists_graph_relations; set ELF_QDRANT_URL to run.");

		return;
	};
	let extractor_payload = serde_json::json!({
		"notes": [{
			"type": "fact",
			"key": "mentorship",
			"text": "Alice mentors Bob.",
			"structured": {
				"relations": [{
					"subject": { "canonical": "Alice" },
					"predicate": "mentors",
					"object": { "value": "Bob" }
				}]
			},
			"importance": 0.8,
			"confidence": 0.9,
			"ttl_days": null,
			"scope_suggestion": "agent_private",
			"evidence": [{ "message_index": 0, "quote": "Alice mentors Bob." }],
			"reason": "test"
		}]
	});
	let providers = Providers::new(
		Arc::new(crate::acceptance::StubEmbedding { vector_dim: 4_096 }),
		Arc::new(crate::acceptance::StubRerank),
		Arc::new(crate::acceptance::SpyExtractor {
			calls: Arc::new(AtomicUsize::new(0)),
			payload: extractor_payload,
		}),
	);
	let collection = test_db.collection_name("elf_acceptance");
	let cfg =
		crate::acceptance::test_config(test_db.dsn().to_string(), qdrant_url, 4_096, collection);
	let service =
		crate::acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	crate::acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

	let response = service
		.add_event(AddEventRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: Some("agent_private".to_string()),
			dry_run: Some(false),
			messages: vec![EventMessage {
				role: "user".to_string(),
				content: "Alice mentors Bob.".to_string(),
				ts: None,
				msg_id: None,
			}],
		})
		.await
		.expect("add_event failed.");

	assert_eq!(response.results.len(), 1);
	assert_eq!(response.results[0].op, NoteOp::Add);

	let note_id = response.results[0].note_id.expect("Expected note_id.");
	let fact_id = graph_fact_id(&service.db.pool).await;
	let fact_count = graph_fact_count(&service.db.pool).await;
	let evidence_count =
		graph_fact_evidence_count_for_note(&service.db.pool, fact_id, note_id).await;

	assert_eq!(fact_count, 1);
	assert_eq!(evidence_count, 1);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
