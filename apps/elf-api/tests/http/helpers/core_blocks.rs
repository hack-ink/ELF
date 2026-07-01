use axum::{Router, body, http::StatusCode};
use tower::util::ServiceExt as _;
use uuid::Uuid;

use crate::helpers::{self, TEST_AGENT_A};

pub(crate) async fn create_core_block(
	admin_app: &Router,
	scope: &str,
	key: &str,
	content: &str,
) -> Uuid {
	let payload = serde_json::json!({
		"scope": scope,
		"key": key,
		"title": "Operating context",
		"content": content,
		"source_ref": {
			"schema": "core_block_source/v1",
			"ref": { "issue": "XY-832" }
		}
	});
	let (status, body) =
		helpers::post_admin_json(admin_app, "/v2/admin/core-blocks", TEST_AGENT_A, payload).await;

	assert_eq!(status, StatusCode::OK);

	Uuid::parse_str(
		body.pointer("/block/block_id")
			.and_then(serde_json::Value::as_str)
			.expect("Missing core block id."),
	)
	.expect("Invalid core block id.")
}

pub(crate) async fn attach_core_block(
	admin_app: &Router,
	block_id: Uuid,
	target_agent_id: &str,
	read_profile: &str,
) -> (StatusCode, serde_json::Value) {
	let payload = serde_json::json!({
		"target_agent_id": target_agent_id,
		"read_profile": read_profile,
		"reason": "Attach fixture block."
	});
	let uri = format!("/v2/admin/core-blocks/{block_id}/attachments");

	helpers::post_admin_json(admin_app, uri, TEST_AGENT_A, payload).await
}

pub(crate) async fn get_core_blocks(
	app: &Router,
	agent_id: &str,
	read_profile: &str,
) -> serde_json::Value {
	let response = app
		.clone()
		.oneshot(helpers::context_request("GET", "/v2/core-blocks", agent_id, read_profile))
		.await
		.expect("Failed to fetch core blocks.");

	assert_eq!(response.status(), StatusCode::OK);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read core blocks response body.");

	serde_json::from_slice(&body).expect("Failed to parse core blocks response.")
}
