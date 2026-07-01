use crate::acceptance::english_only_boundary::setup;
use elf_service::{AddNoteInput, AddNoteRequest, Error};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_non_english_in_add_note() {
	let Some(fixture) = setup::setup_service("english_only_boundary").await else {
		return;
	};
	let request = AddNoteRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "agent_private".to_string(),
		notes: vec![AddNoteInput {
			r#type: "fact".to_string(),
			key: None,
			text: "你好".to_string(),
			structured: None,
			importance: 0.4,
			confidence: 0.9,
			ttl_days: None,
			source_ref: serde_json::json!({}),
			write_policy: None,
		}],
	};
	let result = fixture.service.add_note(request).await;

	match result {
		Err(Error::NonEnglishInput { field }) => {
			assert_eq!(field, "$.notes[0].text");
		},
		other => panic!("Expected NonEnglishInput, got {other:?}"),
	}

	fixture.test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_cyrillic_in_add_note() {
	let Some(fixture) = setup::setup_service("english_only_boundary").await else {
		return;
	};
	let request = AddNoteRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "agent_private".to_string(),
		notes: vec![AddNoteInput {
			r#type: "fact".to_string(),
			key: None,
			text: "Привет мир".to_string(),
			structured: None,
			importance: 0.4,
			confidence: 0.9,
			ttl_days: None,
			source_ref: serde_json::json!({}),
			write_policy: None,
		}],
	};
	let result = fixture.service.add_note(request).await;

	match result {
		Err(Error::NonEnglishInput { field }) => {
			assert_eq!(field, "$.notes[0].text");
		},
		other => panic!("Expected NonEnglishInput, got {other:?}"),
	}

	fixture.test_db.cleanup().await.expect("Failed to cleanup test database.");
}
