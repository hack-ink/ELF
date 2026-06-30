use sqlx::PgExecutor;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::acceptance::{
	self, StubRerank,
	chunk_search::tests_helpers::{self, TestContext},
};
use elf_service::{ElfService, Providers, RelationTemporalStatus, SearchRequest};

async fn insert_graph_entity<'e, E>(
	executor: E,
	entity_id: Uuid,
	canonical: &str,
	kind: Option<&str>,
) where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO graph_entities (
	entity_id,
	tenant_id,
	project_id,
	canonical,
	canonical_norm,
	kind
)
VALUES ($1, $2, $3, $4, $5, $6)",
	)
	.bind(entity_id)
	.bind("t")
	.bind("p")
	.bind(canonical)
	.bind(canonical.to_lowercase())
	.bind(kind)
	.execute(executor)
	.await
	.expect("Failed to insert graph entity.");
}

async fn insert_graph_predicate<'e, E>(executor: E, predicate_id: Uuid, canonical: &str)
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO graph_predicates (
	predicate_id,
	scope_key,
	tenant_id,
	project_id,
	canonical,
	canonical_norm,
	cardinality,
	status
)
VALUES ($1, $2, $3, $4, $5, $6, 'single', 'active')",
	)
	.bind(predicate_id)
	.bind("__project__:p")
	.bind("t")
	.bind("p")
	.bind(canonical)
	.bind(canonical.to_lowercase())
	.execute(executor)
	.await
	.expect("Failed to insert graph predicate.");
}

#[allow(clippy::too_many_arguments)]
async fn insert_graph_fact<'e, E>(
	executor: E,
	fact_id: Uuid,
	subject_entity_id: Uuid,
	predicate: &str,
	predicate_id: Uuid,
	object_value: &str,
	valid_from: OffsetDateTime,
	valid_to: Option<OffsetDateTime>,
) where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO graph_facts (
	fact_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	subject_entity_id,
	predicate,
	predicate_id,
	object_entity_id,
	object_value,
	valid_from,
	valid_to
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, $9, $10, $11)",
	)
	.bind(fact_id)
	.bind("t")
	.bind("p")
	.bind("a")
	.bind("agent_private")
	.bind(subject_entity_id)
	.bind(predicate)
	.bind(predicate_id)
	.bind(object_value)
	.bind(valid_from)
	.bind(valid_to)
	.execute(executor)
	.await
	.expect("Failed to insert graph fact.");
}

async fn insert_graph_fact_evidence<'e, E>(
	executor: E,
	fact_id: Uuid,
	note_id: Uuid,
	created_at: OffsetDateTime,
) where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO graph_fact_evidence (evidence_id, fact_id, note_id, created_at)
VALUES ($1, $2, $3, $4)",
	)
	.bind(Uuid::new_v4())
	.bind(fact_id)
	.bind(note_id)
	.bind(created_at)
	.execute(executor)
	.await
	.expect("Failed to insert graph fact evidence.");
}

async fn setup_graph_context_test(
	test_name: &str,
	providers: Providers,
	max_facts_per_item: u32,
	max_evidence_notes_per_fact: u32,
) -> Option<TestContext> {
	let Some(test_db) = acceptance::test_db().await else {
		eprintln!("Skipping {test_name}; set ELF_PG_DSN to run this test.");

		return None;
	};
	let Some(qdrant_url) = acceptance::test_qdrant_url() else {
		eprintln!("Skipping {test_name}; set ELF_QDRANT_URL to run this test.");

		return None;
	};
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let mut cfg = acceptance::test_config(
		test_db.dsn().to_string(),
		qdrant_url,
		4_096,
		collection,
		docs_collection,
	);

	cfg.search.graph_context.enabled = true;
	cfg.search.graph_context.max_facts_per_item = max_facts_per_item;
	cfg.search.graph_context.max_evidence_notes_per_fact = max_evidence_notes_per_fact;

	let service =
		acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");
	tests_helpers::reset_collection(&service).await;

	let embedding_version = format!(
		"{}:{}:{}",
		service.cfg.providers.embedding.provider_id,
		service.cfg.providers.embedding.model,
		service.cfg.storage.qdrant.vector_dim
	);

	Some(TestContext { service, test_db, embedding_version })
}

async fn seed_relation_context_fixture(
	service: &ElfService,
	embedding_version: &str,
) -> (Uuid, Uuid, Uuid) {
	let now = OffsetDateTime::now_utc();
	let note_id = Uuid::new_v4();
	let note_id_2 = Uuid::new_v4();
	let chunk_id = Uuid::new_v4();
	let chunk_text = "Alice mentors Bob about projects and priorities.";
	let subject_id = Uuid::new_v4();
	let newer_fact_id = Uuid::new_v4();
	let predicate_id = Uuid::new_v4();
	let older_fact_id = Uuid::new_v4();
	let older_fact_valid_from = now - Duration::seconds(10);
	let newer_fact_valid_from = now - Duration::seconds(5);
	let note_1_evidence_created_at = now - Duration::seconds(30);
	let note_2_evidence_created_at = now - Duration::seconds(10);

	tests_helpers::insert_note(&service.db.pool, note_id, chunk_text, embedding_version).await;
	tests_helpers::insert_note(
		&service.db.pool,
		note_id_2,
		"Second note for evidence ordering.",
		embedding_version,
	)
	.await;
	tests_helpers::insert_chunk(
		&service.db.pool,
		chunk_id,
		note_id,
		0,
		0,
		chunk_text.len() as i32,
		chunk_text,
		embedding_version,
	)
	.await;
	tests_helpers::upsert_point(
		service,
		chunk_id,
		note_id,
		0,
		0,
		chunk_text.len() as i32,
		chunk_text,
	)
	.await;

	insert_graph_entity(&service.db.pool, subject_id, "Alice", Some("person")).await;
	insert_graph_predicate(&service.db.pool, predicate_id, "mentors").await;
	insert_graph_fact(
		&service.db.pool,
		older_fact_id,
		subject_id,
		"mentors",
		predicate_id,
		"Bob",
		older_fact_valid_from,
		Some(newer_fact_valid_from),
	)
	.await;
	insert_graph_fact_evidence(
		&service.db.pool,
		older_fact_id,
		note_id,
		note_1_evidence_created_at,
	)
	.await;
	insert_graph_fact(
		&service.db.pool,
		newer_fact_id,
		subject_id,
		"mentors",
		predicate_id,
		"Carol",
		newer_fact_valid_from,
		None,
	)
	.await;
	insert_graph_fact_evidence(
		&service.db.pool,
		newer_fact_id,
		note_id,
		note_1_evidence_created_at,
	)
	.await;
	insert_graph_fact_evidence(
		&service.db.pool,
		newer_fact_id,
		note_id_2,
		note_2_evidence_created_at,
	)
	.await;

	(note_id, newer_fact_id, older_fact_id)
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn search_raw_quick_includes_relation_context_and_respects_fact_bounds() {
	let providers = tests_helpers::build_providers(StubRerank);
	let Some(context) = setup_graph_context_test(
		"search_raw_quick_includes_relation_context_and_respects_fact_bounds",
		providers,
		1,
		1,
	)
	.await
	else {
		return;
	};
	let fixture = seed_relation_context_fixture(&context.service, &context.embedding_version).await;
	let note_id = fixture.0;
	let newer_fact_id = fixture.1;
	let response = context
		.service
		.search_raw_quick(SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			token_id: None,
			read_profile: "private_only".to_string(),
			payload_level: Default::default(),
			query: "Alice".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search failed.");
	let item = response.items.first().expect("Expected search result.");
	let relation_context = item
		.explain
		.relation_context
		.as_ref()
		.expect("Expected relation context in search explain.");

	assert_eq!(relation_context.len(), 1, "Expected relation context to be truncated to one fact.");
	assert_eq!(
		relation_context[0].fact_id, newer_fact_id,
		"Expected the most recent fact after truncation."
	);
	assert_eq!(relation_context[0].object.value.as_deref(), Some("Carol"));
	assert_eq!(relation_context[0].temporal_status, RelationTemporalStatus::Current);
	assert!(relation_context[0].valid_to.is_none());
	assert_eq!(relation_context[0].evidence_note_ids.len(), 1);
	assert_eq!(relation_context[0].evidence_note_ids[0], note_id);

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn search_raw_quick_marks_historical_relation_context() {
	let providers = tests_helpers::build_providers(StubRerank);
	let Some(context) = setup_graph_context_test(
		"search_raw_quick_marks_historical_relation_context",
		providers,
		2,
		2,
	)
	.await
	else {
		return;
	};
	let fixture = seed_relation_context_fixture(&context.service, &context.embedding_version).await;
	let older_fact_id = fixture.2;
	let response = context
		.service
		.search_raw_quick(SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			token_id: None,
			read_profile: "private_only".to_string(),
			payload_level: Default::default(),
			query: "Alice".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search failed.");
	let item = response.items.first().expect("Expected search result.");
	let relation_context = item
		.explain
		.relation_context
		.as_ref()
		.expect("Expected relation context in search explain.");

	assert_eq!(
		relation_context.len(),
		2,
		"Expected current and historical relation facts in context.",
	);
	assert_eq!(relation_context[0].temporal_status, RelationTemporalStatus::Current);

	let historical = relation_context
		.iter()
		.find(|context| context.fact_id == older_fact_id)
		.expect("Expected historical fact in relation context.");

	assert_eq!(historical.object.value.as_deref(), Some("Bob"));
	assert_eq!(historical.temporal_status, RelationTemporalStatus::Historical);
	assert!(historical.valid_to.is_some());

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}
