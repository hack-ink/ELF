use std::{
	collections::HashSet,
	sync::{Arc, atomic::AtomicUsize},
};

use crate::acceptance::{self, SpyExtractor, StubEmbedding, StubRerank};
use elf_service::{
	AddNoteInput, AddNoteRequest, MemoryHistoryGetRequest, NoteOp, NoteProvenanceGetRequest,
	Providers,
};

fn history_request(text: &str, importance: f32) -> AddNoteRequest {
	AddNoteRequest {
		tenant_id: "tenant-history".to_string(),
		project_id: "project-history".to_string(),
		agent_id: "agent-history".to_string(),
		scope: "agent_private".to_string(),
		notes: vec![AddNoteInput {
			r#type: "fact".to_string(),
			key: Some("memory_history_target".to_string()),
			text: text.to_string(),
			structured: None,
			importance,
			confidence: 0.9,
			ttl_days: None,
			source_ref: serde_json::json!({ "schema": "acceptance/history" }),
			write_policy: None,
		}],
	}
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn memory_history_links_versions_and_ignored_decisions() {
	let Some(test_db) = acceptance::test_db().await else {
		eprintln!("Skipping memory_history_links_versions_and_ignored_decisions; set ELF_PG_DSN.");

		return;
	};
	let Some(qdrant_url) = acceptance::test_qdrant_url() else {
		eprintln!(
			"Skipping memory_history_links_versions_and_ignored_decisions; set ELF_QDRANT_URL."
		);

		return;
	};
	let providers = Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(StubRerank),
		Arc::new(SpyExtractor {
			calls: Arc::new(AtomicUsize::new(0)),
			payload: serde_json::json!({ "notes": [] }),
		}),
	);
	let collection = test_db.collection_name("elf_history");
	let docs_collection = test_db.collection_name("elf_history_docs");
	let cfg = acceptance::test_config(
		test_db.dsn().to_string(),
		qdrant_url,
		4_096,
		collection,
		docs_collection,
	);
	let service =
		acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

	let first = service
		.add_note(history_request(
			"Fact: Memory history readback starts with original evidence.",
			0.7,
		))
		.await
		.expect("initial note should be added");
	let note_id = first.results[0].note_id.expect("add should return note id");

	assert_eq!(first.results[0].op, NoteOp::Add);

	let updated = service
		.add_note(history_request("Fact: Memory history readback records updated evidence.", 0.8))
		.await
		.expect("second note should update by key");
	let ignored = service
		.add_note(history_request("Fact: Memory history readback records updated evidence.", 0.8))
		.await
		.expect("third note should be ignored as unchanged");

	assert_eq!(updated.results[0].op, NoteOp::Update);
	assert_eq!(ignored.results[0].op, NoteOp::None);

	let history = service
		.memory_history_get(MemoryHistoryGetRequest {
			tenant_id: "tenant-history".to_string(),
			project_id: "project-history".to_string(),
			note_id,
		})
		.await
		.expect("history should be readable");
	let event_types: HashSet<&str> =
		history.events.iter().map(|event| event.event_type.as_str()).collect();

	assert_eq!(history.schema, "elf.memory_history/v1");
	assert!(event_types.contains("add"));
	assert!(event_types.contains("update"));
	assert!(event_types.contains("ignore"));
	assert!(
		history
			.events
			.iter()
			.filter(|event| matches!(event.event_type.as_str(), "add" | "update"))
			.all(|event| event.related_decision_id.is_some()
				&& event.related_note_version_id.is_some())
	);

	let linked_decision_count: i64 = sqlx::query_scalar(
		"SELECT count(*) FROM memory_ingest_decisions WHERE note_id = $1 AND note_version_id IS NOT NULL",
	)
	.bind(note_id)
	.fetch_one(&service.db.pool)
	.await
	.expect("linked decision count should be queryable");

	assert_eq!(linked_decision_count, 2);

	let provenance = service
		.note_provenance_get(NoteProvenanceGetRequest {
			tenant_id: "tenant-history".to_string(),
			project_id: "project-history".to_string(),
			note_id,
		})
		.await
		.expect("provenance should include history");

	assert_eq!(provenance.history.len(), history.events.len());

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
