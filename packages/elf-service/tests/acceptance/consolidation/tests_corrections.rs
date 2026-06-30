use crate::acceptance::consolidation::tests_helpers;
use elf_service::MemoryCorrectionAction;

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run this test."]
async fn promoted_memory_corrections_suppress_and_restore_recall() {
	let Some(fixture) =
		tests_helpers::setup_service("promoted_memory_corrections_suppress_and_restore_recall")
			.await
	else {
		return;
	};
	let service = &fixture.service;
	let promoted_note_id = tests_helpers::promote_reviewed_memory(service).await;
	let superseded = tests_helpers::apply_memory_correction(
		service,
		promoted_note_id,
		MemoryCorrectionAction::Supersede,
		"Newer reviewed source supersedes the derived memory.",
		"supersede",
		None,
	)
	.await;

	assert_eq!(superseded.status, "deprecated");
	assert!(!tests_helpers::active_list_contains(service, promoted_note_id).await);

	let restored = tests_helpers::apply_memory_correction(
		service,
		promoted_note_id,
		MemoryCorrectionAction::Restore,
		"Rollback to prior approved memory after reviewer audit.",
		"restore",
		superseded.version_id,
	)
	.await;

	assert_eq!(restored.status, "active");
	assert!(tests_helpers::active_list_contains(service, promoted_note_id).await);

	let deleted = tests_helpers::apply_memory_correction(
		service,
		promoted_note_id,
		MemoryCorrectionAction::Delete,
		"Reviewer removed the restored memory from normal recall.",
		"delete",
		None,
	)
	.await;

	assert_eq!(deleted.status, "deleted");
	assert!(!tests_helpers::active_list_contains(service, promoted_note_id).await);

	let event_types = tests_helpers::memory_history_event_types(service, promoted_note_id).await;

	for expected in ["add", "derived", "applied", "superseded", "restored", "delete"] {
		assert!(event_types.iter().any(|event_type| event_type == expected));
	}
}
