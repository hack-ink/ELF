use time::OffsetDateTime;

use crate::entity_memory::TOP_OF_MIND_IMPORTANCE_THRESHOLD;

pub(in crate::entity_memory) fn note_lifecycle(
	status: &str,
	expires_at: Option<OffsetDateTime>,
	as_of: OffsetDateTime,
) -> String {
	match status {
		"active" if expires_at.is_some_and(|expires_at| expires_at <= as_of) => "stale".to_string(),
		"active" => "current".to_string(),
		"deprecated" => "superseded".to_string(),
		"deleted" => "tombstoned".to_string(),
		other => other.to_string(),
	}
}

pub(in crate::entity_memory) fn note_read_bucket(lifecycle: &str, importance: f32) -> String {
	if lifecycle == "current" && importance >= TOP_OF_MIND_IMPORTANCE_THRESHOLD {
		"top_of_mind".to_string()
	} else {
		"background".to_string()
	}
}
