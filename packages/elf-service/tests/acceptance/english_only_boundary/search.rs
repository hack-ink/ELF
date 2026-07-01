use crate::acceptance::english_only_boundary::setup;
use elf_service::{Error, SearchRequest};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_non_english_in_search() {
	let Some(fixture) = setup::setup_service("english_only_boundary").await else {
		return;
	};
	let request = SearchRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		token_id: None,
		read_profile: "private_only".to_string(),
		payload_level: Default::default(),
		query: "안녕하세요".to_string(),
		top_k: Some(5),
		candidate_k: Some(10),
		filter: None,
		record_hits: Some(false),
		ranking: None,
	};
	let result = fixture.service.search(request).await;

	match result {
		Err(Error::NonEnglishInput { field }) => {
			assert_eq!(field, "$.query");
		},
		other => panic!("Expected NonEnglishInput, got {other:?}"),
	}

	fixture.test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_cyrillic_in_search() {
	let Some(fixture) = setup::setup_service("english_only_boundary").await else {
		return;
	};
	let request = SearchRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		token_id: None,
		read_profile: "private_only".to_string(),
		payload_level: Default::default(),
		query: "Привет".to_string(),
		top_k: Some(5),
		candidate_k: Some(10),
		filter: None,
		record_hits: Some(false),
		ranking: None,
	};
	let result = fixture.service.search(request).await;

	match result {
		Err(Error::NonEnglishInput { field }) => {
			assert_eq!(field, "$.query");
		},
		other => panic!("Expected NonEnglishInput, got {other:?}"),
	}

	fixture.test_db.cleanup().await.expect("Failed to cleanup test database.");
}
