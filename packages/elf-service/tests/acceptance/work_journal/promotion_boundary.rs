use uuid::Uuid;

use crate::acceptance::work_journal::helpers;
use elf_service::Error;

#[tokio::test]
async fn work_journal_promotion_boundary_requires_existing_accepted_refs() {
	let Some((service, test_db)) = helpers::work_journal_service().await else {
		return;
	};
	let forged_note_id = Uuid::parse_str("bbbbbbbb-1111-4111-8111-bbbbbbbb1111").expect("uuid");
	let forged_note_request = helpers::request_with_promotion_boundary(
		Uuid::parse_str("bbbbbbbb-2222-4222-8222-bbbbbbbb2222").expect("uuid"),
		serde_json::json!({
			"accepted_memory_authority_ref": helpers::memory_record_ref(forged_note_id),
		}),
	);
	let forged_note_error = service
		.work_journal_entry_create(forged_note_request)
		.await
		.expect_err("syntactically valid but nonexistent memory authority ref should be rejected");

	assert!(matches!(
		forged_note_error,
		Error::InvalidRequest { message } if message.contains("accepted_memory_authority_ref")
	));

	let accepted_note_id = Uuid::parse_str("cccccccc-1111-4111-8111-cccccccc1111").expect("uuid");

	helpers::insert_active_memory_note(&service, accepted_note_id).await;

	let accepted_note_request = helpers::request_with_promotion_boundary(
		Uuid::parse_str("cccccccc-2222-4222-8222-cccccccc2222").expect("uuid"),
		serde_json::json!({
			"accepted_memory_authority_ref": helpers::memory_record_ref(accepted_note_id),
		}),
	);
	let accepted_note = service
		.work_journal_entry_create(accepted_note_request)
		.await
		.expect("existing active memory authority ref should be accepted");

	assert_eq!(
		accepted_note.entry.promotion_boundary["authoritative_memory_allowed"],
		serde_json::json!(true)
	);

	let forged_proposal_id = Uuid::parse_str("dddddddd-1111-4111-8111-dddddddd1111").expect("uuid");
	let forged_proposal_request = helpers::request_with_promotion_boundary(
		Uuid::parse_str("dddddddd-2222-4222-8222-dddddddd2222").expect("uuid"),
		serde_json::json!({
			"accepted_dreaming_review_ref": helpers::dreaming_review_ref(
				forged_proposal_id,
				"applied",
			),
		}),
	);
	let forged_proposal_error = service
		.work_journal_entry_create(forged_proposal_request)
		.await
		.expect_err("syntactically valid but nonexistent dreaming review ref should be rejected");

	assert!(matches!(
		forged_proposal_error,
		Error::InvalidRequest { message } if message.contains("accepted_dreaming_review_ref")
	));

	let accepted_proposal_id =
		Uuid::parse_str("eeeeeeee-1111-4111-8111-eeeeeeee1111").expect("uuid");

	helpers::insert_applied_dreaming_proposal(&service, accepted_proposal_id).await;

	let accepted_proposal_request = helpers::request_with_promotion_boundary(
		Uuid::parse_str("eeeeeeee-2222-4222-8222-eeeeeeee2222").expect("uuid"),
		serde_json::json!({
			"accepted_dreaming_review_ref": helpers::dreaming_review_ref(
				accepted_proposal_id,
				"applied",
			),
		}),
	);
	let accepted_proposal = service
		.work_journal_entry_create(accepted_proposal_request)
		.await
		.expect("existing applied dreaming review ref should be accepted");

	assert_eq!(
		accepted_proposal.entry.promotion_boundary["authoritative_memory_allowed"],
		serde_json::json!(true)
	);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
