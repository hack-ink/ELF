use crate::provenance::types::NoteProvenanceIngestDecision;

pub(in crate::provenance::history) fn version_event_type(op: &str, reason: &str) -> &'static str {
	let reason = reason.to_ascii_lowercase();

	match op {
		"ADD" => "add",
		"UPDATE" => "update",
		"DELETE" if reason.contains("expire") => "expire",
		"DELETE" => "delete",
		"PUBLISH" | "UNPUBLISH" => "related",
		"DEPRECATE" => "superseded",
		"RESTORE" => "restored",
		"INVALIDATE" => "invalidated",
		_ => "related",
	}
}

pub(in crate::provenance::history) fn decision_event_type(
	decision: &NoteProvenanceIngestDecision,
) -> &'static str {
	if decision.policy_decision == "reject" || decision.note_op == "REJECTED" {
		return "reject";
	}
	if decision.policy_decision == "ignore" || decision.note_op == "NONE" {
		return "ignore";
	}

	match decision.note_op.as_str() {
		"ADD" => "add",
		"UPDATE" => "update",
		"DELETE" => "delete",
		_ => "related",
	}
}

pub(in crate::provenance::history) fn proposal_review_event_type(action: &str) -> &'static str {
	match action {
		"apply" => "applied",
		"discard" => "reject",
		"defer" => "defer",
		"approve" => "related",
		_ => "related",
	}
}
