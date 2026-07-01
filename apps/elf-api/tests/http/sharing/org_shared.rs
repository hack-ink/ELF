use axum::http::StatusCode;

use crate::helpers;

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn org_shared_note_is_visible_across_projects() {
	let Some((test_db, app, state, note_id)) =
		helpers::org_shared_note_is_visible_across_projects_fixture().await
	else {
		return;
	};
	let list_before_json = helpers::list_org_shared_notes_as_reader(&app).await;

	assert_eq!(list_before_json["items"].as_array().expect("Missing items array.").len(), 0);

	helpers::publish_org_shared_note_as_reader_can_see(&app, note_id).await;

	let grant_upsert_payload = serde_json::json!({ "grantee_kind": "project" }).to_string();
	let grant_upsert_response = helpers::post_with_authorization_and_json_body(
		&app,
		"/v2/spaces/org_shared/grants",
		"Bearer admin-token",
		&grant_upsert_payload,
		"Failed to build grant upsert request.",
		"Failed to call grant upsert.",
	)
	.await;

	assert_eq!(grant_upsert_response.status(), StatusCode::OK);

	helpers::assert_note_visible_to_project_reader(&app, &state, note_id).await;

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
