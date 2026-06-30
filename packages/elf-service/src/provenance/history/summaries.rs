use crate::provenance::types::NoteProvenanceIngestDecision;

pub(in crate::provenance::history) fn version_summary(event_type: &str, reason: &str) -> String {
	match event_type {
		"add" => format!("Note was added by {reason}."),
		"update" => format!("Note was updated by {reason}."),
		"delete" => format!("Note was deleted by {reason}."),
		"expire" => format!("Note expired through {reason}."),
		"superseded" => format!("Note was superseded by {reason}."),
		"restored" => format!("Note was restored by {reason}."),
		"invalidated" => format!("Note was invalidated by {reason}."),
		_ => format!("Note recorded related transition {reason}."),
	}
}

pub(in crate::provenance::history) fn decision_summary(
	event_type: &str,
	decision: &NoteProvenanceIngestDecision,
) -> String {
	let reason = decision.reason_code.as_deref().unwrap_or("no_reason_code");

	match event_type {
		"ignore" => format!("Ingestion ignored candidate memory with {reason}."),
		"reject" => format!("Ingestion rejected candidate memory with {reason}."),
		_ => format!(
			"Ingestion recorded {} decision for operation {}.",
			decision.policy_decision, decision.note_op
		),
	}
}
