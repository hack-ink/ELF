use super::*;

pub(super) fn public_error_class(err: &Error) -> &'static str {
	match err {
		Error::NonEnglishInput { .. } => "validation_non_english_input",
		Error::InvalidRequest { .. } => "validation_invalid_request",
		Error::ScopeDenied { .. } => "scope_denied",
		Error::NotFound { .. } => "not_found",
		Error::Conflict { .. } => "conflict",
		Error::Provider { .. } => "provider_unavailable",
		Error::Storage { .. } => "storage_unavailable",
		Error::Qdrant { .. } => "vector_store_unavailable",
	}
}

pub(super) fn json_anchor<T>(value: &T) -> Option<String>
where
	T: Serialize + ?Sized,
{
	serde_json::to_value(value).ok().map(|value| value.to_string())
}

pub(super) fn search_item_candidate_key(item: &SearchExplainItem) -> Option<(Uuid, Uuid)> {
	item.chunk_id.map(|chunk_id| candidate_identity(item.note_id, chunk_id))
}

pub(super) fn candidate_identity(note_id: Uuid, chunk_id: Uuid) -> (Uuid, Uuid) {
	(note_id, chunk_id)
}

pub(super) fn candidate_is_selected(
	selected_candidate_keys: &BTreeSet<(Uuid, Uuid)>,
	candidate: &TraceReplayCandidate,
) -> bool {
	selected_candidate_keys.contains(&candidate_identity(candidate.note_id, candidate.chunk_id))
}

pub(super) fn graph_replay_command(
	subject: &str,
	predicate: Option<&GraphQueryPredicateRef>,
) -> String {
	if let Some(predicate) = predicate.and_then(json_anchor) {
		format!("elf_graph_report subject={subject} predicate={predicate} explain=true")
	} else {
		format!("elf_graph_report subject={subject} explain=true")
	}
}

pub(super) fn freshness_from_note_source(source: Option<&NoteDebugSourceRow>) -> String {
	source.map(|row| row.status.clone()).unwrap_or_else(|| "unknown".to_string())
}

pub(super) fn source_ref_from_note_source(source: Option<&NoteDebugSourceRow>) -> Value {
	source.map(|row| serde_json::json!([row.source_ref])).unwrap_or_else(|| serde_json::json!([]))
}

pub(super) fn last_stage_name(stages: &[SearchTrajectoryStage]) -> Option<String> {
	stages.last().map(|stage| stage.stage_name.clone())
}

pub(super) fn knowledge_freshness(item: &KnowledgePageSearchItem) -> String {
	if item.lint_summary.error_count > 0 {
		"lint_error".to_string()
	} else if item.lint_summary.warning_count > 0 {
		"lint_warning".to_string()
	} else if item.trust_state != "clean" {
		item.trust_state.clone()
	} else {
		item.status.clone()
	}
}

pub(super) fn graph_temporal_status(status: crate::RelationTemporalStatus) -> String {
	match status {
		crate::RelationTemporalStatus::Future => "future",
		crate::RelationTemporalStatus::Current => "current",
		crate::RelationTemporalStatus::Historical => "historical",
	}
	.to_string()
}
