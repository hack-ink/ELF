use time::OffsetDateTime;

use crate::recall_debug::{self, BTreeSet, Uuid, tests::tests_helpers};

#[test]
fn compact_replay_artifact_exposes_controls_stage_movement_and_rerank_effects() {
	let trace_id = Uuid::new_v4();
	let selected_note_id = Uuid::new_v4();
	let selected_chunk_id = Uuid::new_v4();
	let dropped_note_id = Uuid::new_v4();
	let dropped_chunk_id = Uuid::new_v4();
	let now = OffsetDateTime::from_unix_timestamp(0).expect("Valid timestamp.");
	let candidates = vec![
		tests_helpers::compact_replay_selected_candidate(selected_note_id, selected_chunk_id, now),
		tests_helpers::compact_replay_dropped_candidate(
			dropped_note_id,
			dropped_chunk_id,
			selected_note_id,
			now,
		),
	];
	let selected =
		BTreeSet::from([recall_debug::candidate_identity(selected_note_id, selected_chunk_id)]);
	let source_refs =
		tests_helpers::compact_replay_source_refs(selected_note_id, dropped_note_id, now);
	let artifact = recall_debug::memory_compact_replay_artifact(
		&tests_helpers::compact_replay_trace(trace_id, now),
		tests_helpers::compact_replay_stages().as_slice(),
		candidates.as_slice(),
		&[],
		&selected,
		&source_refs,
		"elf_admin_trace_bundle_get trace_id=<trace> mode=bounded",
	);

	tests_helpers::assert_compact_replay_artifact(&artifact);
}
