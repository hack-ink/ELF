use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::acceptance::{
	self,
	chunk_search::{
		relation_context::records,
		tests_helpers::{self, TestContext},
	},
};
use elf_service::{ElfService, Providers};

pub(super) struct RelationContextFixture {
	pub(super) note_id: Uuid,
	pub(super) newer_fact_id: Uuid,
	pub(super) older_fact_id: Uuid,
}

pub(super) async fn setup_graph_context_test(
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

pub(super) async fn seed_relation_context_fixture(
	service: &ElfService,
	embedding_version: &str,
) -> RelationContextFixture {
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
	records::insert_graph_entity(&service.db.pool, subject_id, "Alice", Some("person")).await;
	records::insert_graph_predicate(&service.db.pool, predicate_id, "mentors").await;
	records::insert_graph_fact(
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
	records::insert_graph_fact_evidence(
		&service.db.pool,
		older_fact_id,
		note_id,
		note_1_evidence_created_at,
	)
	.await;
	records::insert_graph_fact(
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
	records::insert_graph_fact_evidence(
		&service.db.pool,
		newer_fact_id,
		note_id,
		note_1_evidence_created_at,
	)
	.await;
	records::insert_graph_fact_evidence(
		&service.db.pool,
		newer_fact_id,
		note_id_2,
		note_2_evidence_created_at,
	)
	.await;

	RelationContextFixture { note_id, newer_fact_id, older_fact_id }
}
