use uuid::Uuid;

use crate::helpers::{TEST_PROJECT_ID, TEST_TENANT_ID};
use elf_api::state::AppState;

pub(crate) async fn insert_note(
	state: &AppState,
	note_id: Uuid,
	note_scope: &str,
	note_agent: &str,
	note_text: &str,
) {
	sqlx::query(
		"INSERT INTO memory_notes (
			note_id,
			tenant_id,
			project_id,
			agent_id,
			scope,
			type,
			key,
			text,
			importance,
			confidence,
			status,
			created_at,
			updated_at,
			expires_at,
			embedding_version,
			source_ref
		) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, now(), now(), NULL, $12, $13)",
	)
	.bind(note_id)
	.bind(TEST_TENANT_ID)
	.bind(TEST_PROJECT_ID)
	.bind(note_agent)
	.bind(note_scope)
	.bind("fact")
	.bind(None::<String>)
	.bind(note_text)
	.bind(0.7_f32)
	.bind(0.9_f32)
	.bind("active")
	.bind("v2-test")
	.bind(serde_json::json!({ "source": "integration-test" }))
	.execute(&state.service.db.pool)
	.await
	.expect("Failed to seed memory note.");
}

pub(crate) async fn insert_project_scope_grant(
	state: &AppState,
	owner_agent_id: &str,
	granter_agent_id: &str,
) {
	sqlx::query(
		"INSERT INTO memory_space_grants (
			grant_id,
			tenant_id,
			project_id,
			scope,
			space_owner_agent_id,
			grantee_kind,
			grantee_agent_id,
			granted_by_agent_id
		) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
	)
	.bind(Uuid::new_v4())
	.bind(TEST_TENANT_ID)
	.bind(TEST_PROJECT_ID)
	.bind("project_shared")
	.bind(owner_agent_id)
	.bind("project")
	.bind(None::<String>)
	.bind(granter_agent_id)
	.execute(&state.service.db.pool)
	.await
	.expect("Failed to seed project scope grant.");
}

pub(crate) async fn search_session_count(state: &AppState) -> i64 {
	sqlx::query_scalar("SELECT COUNT(*) FROM search_sessions")
		.fetch_one(&state.service.db.pool)
		.await
		.expect("Failed to count search sessions.")
}

pub(crate) async fn active_project_grant_count(state: &AppState, owner_agent_id: &str) -> i64 {
	sqlx::query_scalar(
		"SELECT COUNT(*) FROM memory_space_grants \
		WHERE tenant_id = $1 AND project_id = $2 AND scope = 'project_shared' \
		AND space_owner_agent_id = $3 AND grantee_kind = 'project' AND revoked_at IS NULL",
	)
	.bind(TEST_TENANT_ID)
	.bind(TEST_PROJECT_ID)
	.bind(owner_agent_id)
	.fetch_one(&state.service.db.pool)
	.await
	.expect("Failed to query project grant count.")
}

pub(crate) async fn note_scope_and_project_id(state: &AppState, note_id: Uuid) -> (String, String) {
	let row: (String, String) = sqlx::query_as(
		"SELECT scope, project_id FROM memory_notes WHERE tenant_id = $1 AND note_id = $2",
	)
	.bind(TEST_TENANT_ID)
	.bind(note_id)
	.fetch_one(&state.service.db.pool)
	.await
	.expect("Failed to query note scope and project id.");

	row
}

pub(crate) async fn active_org_shared_project_grant_count(
	state: &AppState,
	owner_agent_id: &str,
) -> i64 {
	sqlx::query_scalar(
		"SELECT COUNT(*) FROM memory_space_grants \
		WHERE tenant_id = $1 AND project_id = '__org__' AND scope = 'org_shared' \
		AND space_owner_agent_id = $2 AND grantee_kind = 'project' AND revoked_at IS NULL",
	)
	.bind(TEST_TENANT_ID)
	.bind(owner_agent_id)
	.fetch_one(&state.service.db.pool)
	.await
	.expect("Failed to query org_shared project grant count.")
}

pub(crate) async fn active_org_shared_project_grant_count_for_project(
	state: &AppState,
	project_id: &str,
	owner_agent_id: &str,
) -> i64 {
	sqlx::query_scalar(
		"SELECT COUNT(*) FROM memory_space_grants \
		WHERE tenant_id = $1 AND project_id = $2 AND scope = 'org_shared' \
		AND space_owner_agent_id = $3 AND grantee_kind = 'project' AND revoked_at IS NULL",
	)
	.bind(TEST_TENANT_ID)
	.bind(project_id)
	.bind(owner_agent_id)
	.fetch_one(&state.service.db.pool)
	.await
	.expect("Failed to query org_shared project grant count for project.")
}
