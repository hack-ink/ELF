use crate::acceptance::english_only_boundary::setup;
use elf_service::{AddEventRequest, Error, EventMessage};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_non_english_in_add_event() {
	let Some(fixture) = setup::setup_service("english_only_boundary").await else {
		return;
	};
	let request = AddEventRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: Some("agent_private".to_string()),
		dry_run: Some(true),
		ingestion_profile: None,
		messages: vec![EventMessage {
			role: "user".to_string(),
			content: "こんにちは".to_string(),
			ts: None,
			msg_id: None,
			write_policy: None,
		}],
	};
	let result = fixture.service.add_event(request).await;

	match result {
		Err(Error::NonEnglishInput { field }) => {
			assert_eq!(field, "$.messages[0].content");
		},
		other => panic!("Expected NonEnglishInput, got {other:?}"),
	}

	fixture.test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_cyrillic_in_add_event() {
	let Some(fixture) = setup::setup_service("english_only_boundary").await else {
		return;
	};
	let request = AddEventRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: Some("agent_private".to_string()),
		dry_run: Some(true),
		ingestion_profile: None,
		messages: vec![EventMessage {
			role: "user".to_string(),
			content: "Это не английский текст.".to_string(),
			ts: None,
			msg_id: None,
			write_policy: None,
		}],
	};
	let result = fixture.service.add_event(request).await;

	match result {
		Err(Error::NonEnglishInput { field }) => {
			assert_eq!(field, "$.messages[0].content");
		},
		other => panic!("Expected NonEnglishInput, got {other:?}"),
	}

	fixture.test_db.cleanup().await.expect("Failed to cleanup test database.");
}
