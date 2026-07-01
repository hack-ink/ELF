use axum::{
	body::{self, Body},
	http::{Request, StatusCode},
};
use serde_json::Value;
use tower::util::ServiceExt as _;

use elf_api::routes::{self, OPENAPI_JSON_PATH};

pub(crate) fn assert_openapi_method(spec: &Value, path: &str, method: &str) {
	let operation = spec
		.get("paths")
		.and_then(|paths| paths.get(path))
		.and_then(|path_item| path_item.get(method));

	assert!(operation.is_some(), "Missing OpenAPI operation {method} {path}");
}

pub(crate) async fn contract_json() -> Value {
	let app = routes::contract_router::<()>();
	let response = app
		.oneshot(
			Request::builder()
				.uri(OPENAPI_JSON_PATH)
				.body(Body::empty())
				.expect("Failed to build OpenAPI request."),
		)
		.await
		.expect("Failed to call OpenAPI route.");

	assert_eq!(response.status(), StatusCode::OK);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read OpenAPI response body.");

	serde_json::from_slice(&body).expect("Failed to parse OpenAPI response.")
}
