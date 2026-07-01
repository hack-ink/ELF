use std::env;

use axum::{
	Router,
	body::{self, Body},
	http::{Request, Response, StatusCode},
};
use serde_json::Value;
use tower::util::ServiceExt as _;
use tracing::Level;

use crate::helpers::{TEST_PROJECT_ID, TEST_TENANT_ID};
use elf_testkit::TestDatabase;

pub(crate) fn init_test_tracing() {
	let _ = tracing_subscriber::fmt().with_max_level(Level::ERROR).with_test_writer().try_init();
}

pub(crate) fn context_request(
	method: &str,
	uri: impl AsRef<str>,
	agent_id: &str,
	read_profile: &str,
) -> Request<Body> {
	Request::builder()
		.method(method)
		.uri(uri.as_ref())
		.header("content-type", "application/json")
		.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
		.header("X-ELF-Project-Id", TEST_PROJECT_ID)
		.header("X-ELF-Agent-Id", agent_id)
		.header("X-ELF-Read-Profile", read_profile)
		.body(Body::empty())
		.expect("Failed to build context request.")
}

pub(crate) async fn test_env() -> Option<(TestDatabase, String, String)> {
	let base_dsn = match elf_testkit::env_dsn() {
		Some(value) => value,
		None => {
			eprintln!("Skipping HTTP tests; set ELF_PG_DSN to run this test.");

			return None;
		},
	};
	let qdrant_url = match env::var("ELF_QDRANT_GRPC_URL").or_else(|_| env::var("ELF_QDRANT_URL")) {
		Ok(value) => value,
		Err(_) => {
			eprintln!(
				"Skipping HTTP tests; set ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run this test."
			);

			return None;
		},
	};
	let test_db = TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");
	let collection = test_db.collection_name("elf_http");

	Some((test_db, qdrant_url, collection))
}

pub(crate) async fn post_admin_json(
	app: &Router,
	uri: impl AsRef<str>,
	agent_id: &str,
	body: Value,
) -> (StatusCode, Value) {
	let request = Request::builder()
		.method("POST")
		.uri(uri.as_ref())
		.header("content-type", "application/json")
		.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
		.header("X-ELF-Project-Id", TEST_PROJECT_ID)
		.header("X-ELF-Agent-Id", agent_id)
		.body(Body::from(body.to_string()))
		.expect("Failed to build admin JSON request.");
	let response = app.clone().oneshot(request).await.expect("Failed to call admin route.");
	let status = response.status();
	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read admin response body.");

	(status, serde_json::from_slice(&body).expect("Failed to parse admin response."))
}

pub(crate) async fn post_with_authorization_and_json_body(
	app: &Router,
	uri: &str,
	auth: &str,
	payload: &str,
	build_expect: &str,
	call_expect: &str,
) -> Response<Body> {
	app.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri(uri)
				.header("Authorization", auth)
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect(build_expect),
		)
		.await
		.expect(call_expect)
}
