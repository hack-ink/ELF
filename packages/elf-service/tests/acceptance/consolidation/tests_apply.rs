use uuid::Uuid;

use crate::acceptance::consolidation::tests_helpers::{
	self, AGENT_ID, PROJECT_ID, REVIEWER_ID, TENANT_ID,
};
use elf_domain::consolidation::ConsolidationReviewAction;
use elf_service::{ConsolidationProposalReviewRequest, ConsolidationRunGetRequest};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run this test."]
async fn apply_action_is_audited_without_source_rewrite() {
	let Some(fixture) =
		tests_helpers::setup_service("apply_action_is_audited_without_source_rewrite").await
	else {
		return;
	};
	let service = &fixture.service;
	let source_text =
		"Fact: Current consolidation output is derived and never rewrites source notes.";
	let note_id =
		tests_helpers::insert_source_note(service, "consolidation_source_rule", source_text).await;
	let source = tests_helpers::source_ref(note_id);
	let created = tests_helpers::create_run_with_proposals(
		service,
		&source,
		vec![tests_helpers::proposal_input(&source, "derived_note")],
	)
	.await;

	assert_eq!(created.run.status, "pending");
	assert!(created.proposals.is_empty());

	tests_helpers::process_consolidation_worker(service).await;

	let completed = service
		.consolidation_run_get(ConsolidationRunGetRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			run_id: created.run.run_id,
		})
		.await
		.expect("consolidation run should remain readable");
	let materialized = tests_helpers::materialized_proposals(service, created.run.run_id).await;
	let proposal = &materialized.proposals[0];
	let job_status: String =
		sqlx::query_scalar("SELECT status FROM consolidation_run_jobs WHERE job_id = $1")
			.bind(created.job_id)
			.fetch_one(&service.db.pool)
			.await
			.expect("consolidation job should be queryable");

	assert_eq!(completed.status, "completed");
	assert_eq!(job_status, "DONE");
	assert_eq!(materialized.proposals.len(), 1);
	assert_eq!(proposal.review_state, "proposed");
	assert_eq!(proposal.unsupported_claim_flags.as_array().map(Vec::len), Some(1));
	assert_eq!(proposal.contradiction_markers.as_array().map(Vec::len), Some(1));

	let reviewed = service
		.consolidation_proposal_review(ConsolidationProposalReviewRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			reviewer_agent_id: REVIEWER_ID.to_string(),
			proposal_id: proposal.proposal_id,
			review_action: ConsolidationReviewAction::Apply,
			review_comment: Some("Apply reviewed derived proposal.".to_string()),
		})
		.await
		.expect("review action should apply");

	assert_eq!(reviewed.review_state, "applied");
	assert_eq!(reviewed.review_events.len(), 2);
	assert_eq!(reviewed.review_events[0].action, "approve");
	assert_eq!(reviewed.review_events[0].from_review_state, "proposed");
	assert_eq!(reviewed.review_events[0].to_review_state, "approved");
	assert_eq!(reviewed.review_events[1].action, "apply");
	assert_eq!(reviewed.review_events[1].from_review_state, "approved");
	assert_eq!(reviewed.review_events[1].to_review_state, "applied");

	let promoted_note_id = reviewed
		.target_ref
		.get("id")
		.and_then(serde_json::Value::as_str)
		.and_then(|value| Uuid::parse_str(value).ok())
		.expect("applied proposal should point at promoted note");
	let promoted_source_ref: serde_json::Value =
		sqlx::query_scalar("SELECT source_ref FROM memory_notes WHERE note_id = $1")
			.bind(promoted_note_id)
			.fetch_one(&service.db.pool)
			.await
			.expect("promoted memory source ref should be queryable");
	let promoted_status: String =
		sqlx::query_scalar("SELECT status FROM memory_notes WHERE note_id = $1")
			.bind(promoted_note_id)
			.fetch_one(&service.db.pool)
			.await
			.expect("promoted memory status should be queryable");
	let promoted_agent_id: String =
		sqlx::query_scalar("SELECT agent_id FROM memory_notes WHERE note_id = $1")
			.bind(promoted_note_id)
			.fetch_one(&service.db.pool)
			.await
			.expect("promoted memory owner should be queryable");

	assert_eq!(promoted_status, "active");
	assert_eq!(promoted_agent_id, AGENT_ID);
	assert_eq!(promoted_source_ref["schema"], "elf.memory_promotion/v1");
	assert_eq!(
		promoted_source_ref["proposal_id"].as_str().map(str::to_string),
		Some(proposal.proposal_id.to_string())
	);
	assert_eq!(promoted_source_ref["review"]["reviewer_agent_id"], REVIEWER_ID);

	let stored_text: String =
		sqlx::query_scalar("SELECT text FROM memory_notes WHERE note_id = $1")
			.bind(note_id)
			.fetch_one(&service.db.pool)
			.await
			.expect("source note should still exist");
	let version_count: i64 =
		sqlx::query_scalar("SELECT count(*) FROM memory_note_versions WHERE note_id = $1")
			.bind(note_id)
			.fetch_one(&service.db.pool)
			.await
			.expect("source note versions should be queryable");

	assert_eq!(stored_text, source_text);
	assert_eq!(version_count, 1);
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run this test."]
async fn apply_project_shared_memory_creates_owner_grant() {
	let Some(fixture) =
		tests_helpers::setup_service("apply_project_shared_memory_creates_owner_grant").await
	else {
		return;
	};
	let service = &fixture.service;
	let note_id = tests_helpers::insert_source_note(
		service,
		"consolidation_project_shared_source",
		"Fact: Shared memory promotions must preserve project grant semantics.",
	)
	.await;
	let source = tests_helpers::source_ref(note_id);
	let proposal = tests_helpers::proposal_input_with_payload(
		&source,
		"derived_note",
		serde_json::json!({
			"type": "fact",
			"scope": "project_shared",
			"text": "Fact: Project-shared promoted memories keep project grants."
		}),
	);
	let created = tests_helpers::create_run_with_proposals(service, &source, vec![proposal]).await;

	tests_helpers::process_consolidation_worker(service).await;

	let materialized = tests_helpers::materialized_proposals(service, created.run.run_id).await;
	let reviewed = service
		.consolidation_proposal_review(ConsolidationProposalReviewRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			reviewer_agent_id: REVIEWER_ID.to_string(),
			proposal_id: materialized.proposals[0].proposal_id,
			review_action: ConsolidationReviewAction::Apply,
			review_comment: Some("Apply reviewed project-shared memory.".to_string()),
		})
		.await
		.expect("project-shared review action should promote memory");
	let promoted_note_id = reviewed
		.target_ref
		.get("id")
		.and_then(serde_json::Value::as_str)
		.and_then(|value| Uuid::parse_str(value).ok())
		.expect("applied proposal should point at promoted note");
	let promoted: (String, String, String) =
		sqlx::query_as("SELECT project_id, agent_id, scope FROM memory_notes WHERE note_id = $1")
			.bind(promoted_note_id)
			.fetch_one(&service.db.pool)
			.await
			.expect("promoted memory should be queryable");
	let grant_count: i64 = sqlx::query_scalar(
		"\
SELECT count(*)
FROM memory_space_grants
WHERE tenant_id = $1
	AND project_id = $2
	AND scope = 'project_shared'
	AND space_owner_agent_id = $3
	AND grantee_kind = 'project'
	AND revoked_at IS NULL",
	)
	.bind(TENANT_ID)
	.bind(PROJECT_ID)
	.bind(AGENT_ID)
	.fetch_one(&service.db.pool)
	.await
	.expect("project grant should be queryable");

	assert_eq!(
		promoted,
		(PROJECT_ID.to_string(), AGENT_ID.to_string(), "project_shared".to_string())
	);
	assert_eq!(grant_count, 1);
}
