use std::collections::HashSet;

use crate::access;

pub(in crate::entity_memory) fn row_read_allowed(
	owner_agent_id: &str,
	scope: &str,
	requester_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<access::SharedSpaceGrantKey>,
) -> bool {
	if !allowed_scopes.iter().any(|allowed| allowed == scope) {
		return false;
	}
	if scope == "agent_private" {
		return owner_agent_id == requester_agent_id;
	}
	if !matches!(scope, "project_shared" | "org_shared") {
		return false;
	}
	if owner_agent_id == requester_agent_id {
		return true;
	}

	shared_grants.contains(&access::SharedSpaceGrantKey {
		scope: scope.to_string(),
		space_owner_agent_id: owner_agent_id.to_string(),
	})
}
