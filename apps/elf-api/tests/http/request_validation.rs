use axum::{
	body::{self, Body},
	http::{Request, StatusCode},
};
use serde_json::Value;
use tower::util::ServiceExt as _;
use uuid::Uuid;

use crate::{TEST_AGENT_A, TEST_PROJECT_ID, TEST_TENANT_ID};
use elf_api::{routes, state::AppState};

fn payload_level_source_ref() -> Value {
	serde_json::json!({
		"schema": "note_source_ref/v1",
		"locator": {
			"document_id": Uuid::new_v4().to_string(),
			"chunk_id": Uuid::new_v4().to_string(),
			"revision": "payload-shaping-contract-test"
		},
		"metadata": {
			"heavy_field": "This field should be hidden when payload_level is below l2."
		}
	})
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn health_ok() {
	let Some((test_db, qdrant_url, collection)) = crate::test_env().await else {
		return;
	};
	let config = crate::test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state.clone());
	let _ = routes::admin_router(state);
	let response = app
		.oneshot(
			Request::builder()
				.uri("/health")
				.body(Body::empty())
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call /health.");

	assert_eq!(response.status(), StatusCode::OK);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn rejects_non_english_in_add_note() {
	let Some((test_db, qdrant_url, collection)) = crate::test_env().await else {
		return;
	};
	let config = crate::test_config(test_db.dsn().to_string(), qdrant_url, collection);
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
	let Some((test_db, qdrant_url, collection)) = crate::test_env().await else {
		return;
	};
	let config = crate::test_config(test_db.dsn().to_string(), qdrant_url, collection);
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
	let Some((test_db, qdrant_url, collection)) = crate::test_env().await else { return };
	let config = crate::test_config(test_db.dsn().to_string(), qdrant_url, collection);
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
	let Some((test_db, qdrant_url, collection)) = crate::test_env().await else { return };
	let config = crate::test_config(test_db.dsn().to_string(), qdrant_url, collection);
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
	let Some((test_db, qdrant_url, collection)) = crate::test_env().await else {
		return;
	};
	let config = crate::test_config(test_db.dsn().to_string(), qdrant_url, collection);
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
	let Some((test_db, qdrant_url, collection)) = crate::test_env().await else {
		return;
	};
	let config = crate::test_config(test_db.dsn().to_string(), qdrant_url, collection);
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

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn searches_notes_payload_level_shapes_source_ref_and_structured() {
	let Some((test_db, qdrant_url, collection)) = crate::test_env().await else {
		return;
	};
	let config = crate::test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state.clone());
	let source_ref = payload_level_source_ref();
	let structured_summary = "Compact structured summary used for payload-level l1 and l2 shaping.";
	let note_text =
		"Payload shaping note used in contract tests for search details output shaping.";
	let note_id =
		crate::create_note_for_payload_level_tests(&app, &state, note_text, source_ref.clone())
			.await;

	crate::insert_note_summary_field(&state, note_id, structured_summary).await;

	let search_response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/searches")
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_A)
				.header("X-ELF-Read-Profile", "private_only")
				.header("content-type", "application/json")
				.body(Body::from(
					serde_json::json!({
						"mode": "quick_find",
						"query": "payload shaping",
						"top_k": 5,
						"candidate_k": 10,
					})
					.to_string(),
				))
				.expect("Failed to build searches request."),
		)
		.await
		.expect("Failed to call searches.");

	assert_eq!(search_response.status(), StatusCode::OK);

	let search_body = body::to_bytes(search_response.into_body(), usize::MAX)
		.await
		.expect("Failed to read searches response body.");
	let search_json: Value =
		serde_json::from_slice(&search_body).expect("Failed to parse searches response.");
	let trajectory = &search_json["trajectory_summary"];

	if !trajectory.is_null() {
		assert!(trajectory.is_object());
		assert!(trajectory.get("stages").is_some());
	}

	let search_id = Uuid::parse_str(
		search_json["search_id"].as_str().expect("Missing search_id in searches response."),
	)
	.expect("Invalid search_id value.");
	let notes_l0 =
		crate::fetch_search_notes_for_payload_level(&app, search_id, note_id, "l0").await;
	let notes_l1 =
		crate::fetch_search_notes_for_payload_level(&app, search_id, note_id, "l1").await;
	let notes_l2 =
		crate::fetch_search_notes_for_payload_level(&app, search_id, note_id, "l2").await;
	let search_get_response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("GET")
				.uri(format!("/v2/searches/{search_id}"))
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_A)
				.header("X-ELF-Read-Profile", "private_only")
				.body(Body::empty())
				.expect("Failed to build searches get request."),
		)
		.await
		.expect("Failed to call searches get.");

	assert_eq!(search_get_response.status(), StatusCode::OK);

	let search_get_body = body::to_bytes(search_get_response.into_body(), usize::MAX)
		.await
		.expect("Failed to read searches get response body.");
	let search_get_json: Value =
		serde_json::from_slice(&search_get_body).expect("Failed to parse searches get response.");
	let search_get_trajectory = &search_get_json["trajectory_summary"];

	if !search_get_trajectory.is_null() {
		assert!(search_get_trajectory.is_object());
		assert!(search_get_trajectory.get("stages").is_some());
	}

	let notes_l0_text = notes_l0["text"].as_str().expect("Missing l0 text.");
	let notes_l1_text = notes_l1["text"].as_str().expect("Missing l1 text.");
	let notes_l2_text = notes_l2["text"].as_str().expect("Missing l2 text.");

	assert_eq!(notes_l0["source_ref"], serde_json::json!({}));
	assert_eq!(notes_l1["source_ref"], serde_json::json!({}));
	assert_eq!(notes_l2["source_ref"], source_ref);
	assert!(notes_l0["structured"].is_null());
	assert!(notes_l1["structured"].is_object());
	assert!(notes_l2["structured"].is_object());
	assert!(notes_l0_text.len() <= 240);
	assert_eq!(notes_l0_text, note_text);
	assert_eq!(notes_l1_text, structured_summary);
	assert_eq!(notes_l2_text, note_text);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn admin_searches_raw_payload_level_shapes_source_ref() {
	let Some((test_db, qdrant_url, collection)) = crate::test_env().await else {
		return;
	};
	let config = crate::test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state.clone());
	let admin_app = routes::admin_router(state.clone());
	let source_ref = serde_json::json!({
		"schema": "note_source_ref/v1",
		"locator": {
			"document_id": Uuid::new_v4().to_string(),
			"chunk_id": Uuid::new_v4().to_string(),
			"revision": "admin-raw-contract-test"
		},
		"metadata": {
			"heavy_field": "This field should be hidden when payload_level is below l2."
		}
	});
	let note_text =
		"Admin raw search payload shaping contract note. This long note should be indexed.";
	let _note_id =
		crate::create_note_for_payload_level_tests(&app, &state, note_text, source_ref.clone())
			.await;
	let raw_l0 =
		crate::fetch_admin_search_raw_source_ref(&admin_app, "payload shaping", "l0").await;
	let raw_l1 =
		crate::fetch_admin_search_raw_source_ref(&admin_app, "payload shaping", "l1").await;
	let raw_l2 =
		crate::fetch_admin_search_raw_source_ref(&admin_app, "payload shaping", "l2").await;

	assert_eq!(raw_l0, serde_json::json!({}));
	assert_eq!(raw_l1, serde_json::json!({}));
	assert_eq!(raw_l2, source_ref);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
