use axum::{
	body::{self, Body},
	http::{Request, StatusCode},
};
use tower::util::ServiceExt as _;

use crate::helpers;
use elf_api::routes::{self, SCALAR_DOCS_PATH};

#[tokio::test]
async fn openapi_json_route_serves_generated_contract() {
	let spec = helpers::contract_json().await;

	assert_eq!(spec["info"]["title"], "ELF API");
	assert!(spec.get("request_id").is_none());

	helpers::assert_openapi_method(&spec, "/health", "get");
	helpers::assert_openapi_method(&spec, "/v2/notes/ingest", "post");
	helpers::assert_openapi_method(&spec, "/v2/events/ingest", "post");
	helpers::assert_openapi_method(&spec, "/v2/core-blocks", "get");
	helpers::assert_openapi_method(&spec, "/v2/entity-memory", "get");
	helpers::assert_openapi_method(&spec, "/v2/docs/search/l0", "post");
	helpers::assert_openapi_method(&spec, "/v2/work-journal/entries", "post");
	helpers::assert_openapi_method(&spec, "/v2/work-journal/entries/{entry_id}", "get");
	helpers::assert_openapi_method(&spec, "/v2/work-journal/readback", "post");
	helpers::assert_openapi_method(&spec, "/v2/searches/{search_id}/notes", "post");
	helpers::assert_openapi_method(&spec, "/v2/admin/core-blocks", "post");
	helpers::assert_openapi_method(&spec, "/v2/admin/core-blocks/{block_id}/attachments", "post");
	helpers::assert_openapi_method(
		&spec,
		"/v2/admin/core-blocks/attachments/{attachment_id}",
		"delete",
	);
	helpers::assert_openapi_method(&spec, "/v2/admin/docs/{doc_id}", "get");
	helpers::assert_openapi_method(&spec, "/v2/admin/docs/search/l0", "post");
	helpers::assert_openapi_method(&spec, "/v2/admin/docs/excerpts", "post");
	helpers::assert_openapi_method(&spec, "/v2/graph/report", "post");
	helpers::assert_openapi_method(&spec, "/v2/admin/searches/raw", "post");
	helpers::assert_openapi_method(&spec, "/v2/admin/events/ingestion-profiles/default", "get");
	helpers::assert_openapi_method(&spec, "/v2/admin/events/ingestion-profiles/default", "put");
	helpers::assert_openapi_method(&spec, "/v2/admin/consolidation/runs", "post");
	helpers::assert_openapi_method(&spec, "/v2/admin/consolidation/runs", "get");
	helpers::assert_openapi_method(&spec, "/v2/admin/consolidation/runs/{run_id}", "get");
	helpers::assert_openapi_method(&spec, "/v2/admin/consolidation/proposals", "get");
	helpers::assert_openapi_method(&spec, "/v2/admin/consolidation/proposals/{proposal_id}", "get");
	helpers::assert_openapi_method(
		&spec,
		"/v2/admin/consolidation/proposals/{proposal_id}/review",
		"post",
	);
	helpers::assert_openapi_method(&spec, "/v2/admin/notes/{note_id}/corrections", "post");
	helpers::assert_openapi_method(&spec, "/v2/admin/knowledge/pages/rebuild", "post");
	helpers::assert_openapi_method(&spec, "/v2/admin/knowledge/pages", "get");
	helpers::assert_openapi_method(&spec, "/v2/admin/knowledge/pages/search", "post");
	helpers::assert_openapi_method(&spec, "/v2/admin/knowledge/pages/{page_id}", "get");
	helpers::assert_openapi_method(&spec, "/v2/admin/knowledge/pages/{page_id}/lint", "post");
}

#[tokio::test]
async fn scalar_docs_route_serves_api_reference_html() {
	let app = routes::contract_router::<()>();
	let response = app
		.oneshot(
			Request::builder()
				.uri(SCALAR_DOCS_PATH)
				.body(Body::empty())
				.expect("Failed to build Scalar docs request."),
		)
		.await
		.expect("Failed to call Scalar docs route.");

	assert_eq!(response.status(), StatusCode::OK);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read Scalar docs response body.");
	let html = String::from_utf8(body.to_vec()).expect("Scalar docs response was not UTF-8.");

	assert!(html.contains("@scalar/api-reference"));
	assert!(html.contains("/v2/admin/events/ingestion-profiles/default"));
	assert!(html.contains("/v2/admin/consolidation/proposals"));
	assert!(html.contains("/v2/admin/docs/search/l0"));
	assert!(html.contains("/v2/admin/knowledge/pages"));
	assert!(html.contains("/v2/admin/knowledge/pages/search"));
	assert!(html.contains("/v2/work-journal/readback"));
}

#[tokio::test]
async fn openapi_includes_default_ingestion_profile_get_put_contract() {
	let spec = helpers::contract_json().await;
	let default_path = &spec["paths"]["/v2/admin/events/ingestion-profiles/default"];
	let get_schema_ref =
		default_path["get"]["responses"]["200"]["content"]["application/json"]["schema"]["$ref"]
			.as_str()
			.expect("Missing default profile GET response schema ref.");
	let put_request_schema_ref = default_path["put"]["requestBody"]["content"]["application/json"]
		["schema"]["$ref"]
		.as_str()
		.expect("Missing default profile PUT request schema ref.");
	let put_response_schema_ref =
		default_path["put"]["responses"]["200"]["content"]["application/json"]["schema"]["$ref"]
			.as_str()
			.expect("Missing default profile PUT response schema ref.");

	assert!(get_schema_ref.ends_with("/AdminIngestionProfileDefaultResponseV2"));
	assert!(put_request_schema_ref.ends_with("/AdminIngestionProfileDefaultSetBody"));
	assert!(put_response_schema_ref.ends_with("/AdminIngestionProfileDefaultResponseV2"));

	let schemas = &spec["components"]["schemas"];
	let request_schema = &schemas["AdminIngestionProfileDefaultSetBody"];
	let response_schema = &schemas["AdminIngestionProfileDefaultResponseV2"];

	assert!(request_schema["properties"].get("profile_id").is_some());
	assert!(request_schema["properties"].get("version").is_some());
	assert!(
		request_schema["required"]
			.as_array()
			.expect("Missing request required fields")
			.contains(&serde_json::json!("profile_id"))
	);
	assert!(response_schema["properties"].get("profile_id").is_some());
	assert!(response_schema["properties"].get("version").is_some());
	assert!(response_schema["properties"].get("updated_at").is_some());
}
