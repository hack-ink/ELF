use super::*;

pub(super) fn note_debug_source_pair(note: MemoryNote) -> (Uuid, NoteDebugSourceRow) {
	(
		note.note_id,
		NoteDebugSourceRow {
			status: note.status,
			source_ref: note.source_ref,
			updated_at: note.updated_at,
		},
	)
}

pub(super) fn note_debug_read_allowed(
	note: &MemoryNote,
	requester_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<SharedSpaceGrantKey>,
	now: OffsetDateTime,
) -> bool {
	if note.status != "active" || note.expires_at.is_some_and(|expires_at| expires_at <= now) {
		return false;
	}
	if !allowed_scopes.iter().any(|scope| scope == &note.scope) {
		return false;
	}
	if note.scope == "agent_private" {
		return note.agent_id == requester_agent_id;
	}
	if !matches!(note.scope.as_str(), "project_shared" | "org_shared") {
		return false;
	}
	if note.agent_id == requester_agent_id {
		return true;
	}

	shared_grants.contains(&SharedSpaceGrantKey {
		scope: note.scope.clone(),
		space_owner_agent_id: note.agent_id.clone(),
	})
}
