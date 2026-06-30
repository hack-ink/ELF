use crate::acceptance::consolidation::tests_helpers::{self, AGENT_ID, PROJECT_ID, TENANT_ID};
use elf_domain::consolidation::ConsolidationReviewAction;
use elf_service::{ConsolidationProposalGetRequest, ConsolidationProposalReviewRequest};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run this test."]
async fn discard_and_defer_actions_remain_auditable() {
	let Some(fixture) =
		tests_helpers::setup_service("discard_and_defer_actions_remain_auditable").await
	else {
		return;
	};
	let service = &fixture.service;
	let note_id = tests_helpers::insert_source_note(
		service,
		"consolidation_review_actions",
		"Fact: Discarded and deferred proposals remain auditable.",
	)
	.await;
	let source = tests_helpers::source_ref(note_id);
	let created = tests_helpers::create_run_with_proposals(
		service,
		&source,
		vec![
			tests_helpers::proposal_input(&source, "contradiction_report"),
			tests_helpers::proposal_input(&source, "preference_candidate"),
		],
	)
	.await;

	tests_helpers::process_consolidation_worker(service).await;

	let materialized = tests_helpers::materialized_proposals(service, created.run.run_id).await;
	let discarded_id = tests_helpers::proposal_id_by_kind(&materialized, "contradiction_report");
	let deferred_id = tests_helpers::proposal_id_by_kind(&materialized, "preference_candidate");
	let discarded = service
		.consolidation_proposal_review(ConsolidationProposalReviewRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			reviewer_agent_id: AGENT_ID.to_string(),
			proposal_id: discarded_id,
			review_action: ConsolidationReviewAction::Discard,
			review_comment: Some("Discard stale synthesis.".to_string()),
		})
		.await
		.expect("discard should be allowed");
	let deferred = service
		.consolidation_proposal_review(ConsolidationProposalReviewRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			reviewer_agent_id: AGENT_ID.to_string(),
			proposal_id: deferred_id,
			review_action: ConsolidationReviewAction::Defer,
			review_comment: Some("Defer until more evidence is available.".to_string()),
		})
		.await
		.expect("defer should be allowed");
	let deferred_readback = service
		.consolidation_proposal_get(ConsolidationProposalGetRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			proposal_id: deferred_id,
		})
		.await
		.expect("deferred proposal should remain readable");

	assert_eq!(discarded.review_state, "rejected");
	assert_eq!(discarded.review_events.len(), 1);
	assert_eq!(discarded.review_events[0].action, "discard");
	assert_eq!(deferred.review_state, "archived");
	assert_eq!(deferred.review_events.len(), 1);
	assert_eq!(deferred.review_events[0].action, "defer");
	assert_eq!(deferred_readback.review_events.len(), 1);
	assert_eq!(deferred_readback.review_events[0].to_review_state, "archived");
}
