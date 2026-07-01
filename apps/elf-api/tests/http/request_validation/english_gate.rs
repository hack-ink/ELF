use axum::{
	body::{self, Body},
	http::{Request, StatusCode},
};
use serde_json::Value;
use tower::util::ServiceExt as _;

use crate::helpers;
use elf_api::{routes, state::AppState};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn rejects_non_english_in_add_note() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else {
		return;
	};
	let config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);
	let payload = serde_json::json!({
		"scope": "agent_private",
		"notes": [{
			"type": "fact",
			"key": null,
			"text": "你好",
			"importance": 0.5,
			"confidence": 0.9,
			"ttl_days": null,
			"source_ref": {}
		}]
	});
	let response = app
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/notes/ingest")
				.header("X-ELF-Tenant-Id", "t")
				.header("X-ELF-Project-Id", "p")
				.header("X-ELF-Agent-Id", "a")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call add_note.");

	assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read response body.");
	let json: Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["error_code"], "NON_ENGLISH_INPUT");
	assert_eq!(json["fields"][0], "$.notes[0].text");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn rejects_cyrillic_in_add_note() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else {
		return;
	};
	let config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);
	let payload = serde_json::json!({
		"scope": "agent_private",
		"notes": [{
			"type": "fact",
			"key": null,
			"text": "Привет мир",
			"importance": 0.5,
			"confidence": 0.9,
			"ttl_days": null,
			"source_ref": {}
		}]
	});
	let response = app
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/notes/ingest")
				.header("X-ELF-Tenant-Id", "t")
				.header("X-ELF-Project-Id", "p")
				.header("X-ELF-Agent-Id", "a")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call add_note.");

	assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read response body.");
	let json: Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["error_code"], "NON_ENGLISH_INPUT");
	assert_eq!(json["fields"][0], "$.notes[0].text");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn rejects_non_english_in_add_event() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else {
		return;
	};
	let config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);
	let payload = serde_json::json!({
		"scope": "agent_private",
		"dry_run": true,
		"messages": [{
			"role": "user",
			"content": "こんにちは"
		}]
	});
	let response = app
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/events/ingest")
				.header("X-ELF-Tenant-Id", "t")
				.header("X-ELF-Project-Id", "p")
				.header("X-ELF-Agent-Id", "a")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call add_event.");

	assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read response body.");
	let json: Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["error_code"], "NON_ENGLISH_INPUT");
	assert_eq!(json["fields"][0], "$.messages[0].content");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn rejects_cyrillic_in_add_event() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else {
		return;
	};
	let config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);
	let payload = serde_json::json!({
		"scope": "agent_private",
		"dry_run": true,
		"messages": [{
			"role": "user",
			"content": "Это не английский текст."
		}]
	});
	let response = app
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/events/ingest")
				.header("X-ELF-Tenant-Id", "t")
				.header("X-ELF-Project-Id", "p")
				.header("X-ELF-Agent-Id", "a")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call add_event.");

	assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read response body.");
	let json: Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["error_code"], "NON_ENGLISH_INPUT");
	assert_eq!(json["fields"][0], "$.messages[0].content");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn rejects_non_english_in_search() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else {
		return;
	};
	let config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);

	for mode in ["quick_find", "planned_search"] {
		let payload = serde_json::json!({
			"mode": mode,
			"query": "안녕하세요",
			"top_k": 5,
			"candidate_k": 10,
		});
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.method("POST")
					.uri("/v2/searches")
					.header("X-ELF-Tenant-Id", "t")
					.header("X-ELF-Project-Id", "p")
					.header("X-ELF-Agent-Id", "a")
					.header("X-ELF-Read-Profile", "private_only")
					.header("content-type", "application/json")
					.body(Body::from(payload.to_string()))
					.expect("Failed to build request."),
			)
			.await
			.expect("Failed to call search.");

		assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

		let body = body::to_bytes(response.into_body(), usize::MAX)
			.await
			.expect("Failed to read response body.");
		let json: Value = serde_json::from_slice(&body).expect("Failed to parse response.");

		assert_eq!(json["error_code"], "NON_ENGLISH_INPUT");
		assert_eq!(json["fields"][0], "$.query");
	}

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn rejects_cyrillic_in_search() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else {
		return;
	};
	let config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);

	for mode in ["quick_find", "planned_search"] {
		let payload = serde_json::json!({
			"mode": mode,
			"query": "Привет",
			"top_k": 5,
			"candidate_k": 10,
		});
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.method("POST")
					.uri("/v2/searches")
					.header("X-ELF-Tenant-Id", "t")
					.header("X-ELF-Project-Id", "p")
					.header("X-ELF-Agent-Id", "a")
					.header("X-ELF-Read-Profile", "private_only")
					.header("content-type", "application/json")
					.body(Body::from(payload.to_string()))
					.expect("Failed to build request."),
			)
			.await
			.expect("Failed to call search.");

		assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

		let body = body::to_bytes(response.into_body(), usize::MAX)
			.await
			.expect("Failed to read response body.");
		let json: Value = serde_json::from_slice(&body).expect("Failed to parse response.");

		assert_eq!(json["error_code"], "NON_ENGLISH_INPUT");
		assert_eq!(json["fields"][0], "$.query");
	}

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
