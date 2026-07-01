use axum::http::HeaderMap;
use serde_json::Value;
use uuid::Uuid;

use crate::routes::{self, HEADER_REQUEST_ID};

#[test]
fn parse_request_id_from_headers_generates_when_missing() {
	let headers = HeaderMap::new();
	let request_id = routes::parse_request_id_from_headers(&headers)
		.expect("Expected a generated request ID when header is missing.");

	assert_ne!(request_id.to_string(), Uuid::nil().to_string());
}

#[test]
fn parse_request_id_from_headers_rejects_invalid() {
	let mut headers = HeaderMap::new();

	headers.insert(HEADER_REQUEST_ID, "not-a-uuid".parse().expect("invalid request_id"));

	let err = routes::parse_request_id_from_headers(&headers)
		.expect_err("Expected invalid request_id to be rejected.");

	assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
	assert_eq!(err.error_code, "INVALID_REQUEST");
	assert_eq!(err.fields, Some(vec![format!("$.headers.{HEADER_REQUEST_ID}")]));
}

#[test]
fn inject_request_id_into_json_body_adds_request_id_to_object() {
	let request_id = Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").expect("valid uuid");
	let body = serde_json::json!({"note_id":"abc","status":"ok"}).to_string();
	let response_body = routes::inject_request_id_into_json_body(body.as_bytes(), &request_id)
		.expect("Expected request_id field to be injected.");
	let response_value =
		serde_json::from_slice::<Value>(&response_body).expect("Expected valid JSON");

	assert_eq!(response_value["request_id"], request_id.to_string());
}

#[test]
fn inject_request_id_into_json_body_skips_non_object() {
	let request_id = Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").expect("valid uuid");
	let body = serde_json::json!(["a", "b", "c"]).to_string();

	assert!(routes::inject_request_id_into_json_body(body.as_bytes(), &request_id).is_none());
}
