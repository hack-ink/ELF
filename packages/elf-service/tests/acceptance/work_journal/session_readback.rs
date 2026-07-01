use uuid::Uuid;

use crate::acceptance::work_journal::helpers;
use elf_service::{WorkJournalEntryGetRequest, WorkJournalSessionReadbackRequest};

#[tokio::test]
async fn work_journal_persists_redacted_source_adjacent_session_readback() {
	let Some((service, test_db)) = helpers::work_journal_service().await else {
		return;
	};
	let entry_id = Uuid::parse_str("aaaaaaaa-1111-4111-8111-aaaaaaaa1111").expect("uuid");
	let created = service
		.work_journal_entry_create(helpers::work_journal_entry_request(entry_id))
		.await
		.expect("journal entry should persist");

	assert_eq!(created.entry.entry_id, entry_id);
	assert!(created.entry.body.contains("[redacted credential]"));
	assert!(!created.entry.body.contains("abcdef123"));
	assert_eq!(
		created.entry.promotion_boundary["authoritative_memory_allowed"],
		serde_json::json!(false)
	);

	let fetched = service
		.work_journal_entry_get(WorkJournalEntryGetRequest {
			tenant_id: "tenant".to_string(),
			project_id: "project".to_string(),
			agent_id: "agent-a".to_string(),
			read_profile: "private_only".to_string(),
			entry_id,
		})
		.await
		.expect("journal entry should be readable");

	assert_eq!(fetched.entry_id, entry_id);
	assert_eq!(fetched.source_refs.len(), 1);

	let readback = service
		.work_journal_session_readback(WorkJournalSessionReadbackRequest {
			tenant_id: "tenant".to_string(),
			project_id: "project".to_string(),
			agent_id: "agent-a".to_string(),
			read_profile: "private_only".to_string(),
			session_id: "xy-1117-session".to_string(),
			families: vec![],
			limit: Some(10),
		})
		.await
		.expect("session readback should load journal evidence");

	assert_eq!(readback.items.len(), 1);

	let where_stopped = readback.where_stopped.expect("where_stopped should be present");

	assert_eq!(where_stopped.latest_entry_id, entry_id);
	assert_eq!(
		where_stopped.explicit_next_steps,
		vec!["Run the Work Journal validation tests.".to_string()]
	);
	assert_eq!(
		where_stopped.promotion_boundary["authoritative_memory_allowed"],
		serde_json::json!(false)
	);

	let memory_count: i64 = sqlx::query_scalar("SELECT count(*) FROM memory_notes")
		.fetch_one(&service.db.pool)
		.await
		.expect("memory_notes count should query");
	let outbox_count: i64 = sqlx::query_scalar("SELECT count(*) FROM indexing_outbox")
		.fetch_one(&service.db.pool)
		.await
		.expect("indexing_outbox count should query");

	assert_eq!(memory_count, 0, "Work Journal must not create authoritative memory notes");
	assert_eq!(outbox_count, 0, "Work Journal must not enqueue memory indexing");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
