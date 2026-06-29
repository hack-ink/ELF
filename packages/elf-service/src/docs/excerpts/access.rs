use crate::docs::{Error, HashSet, PointIdOptions, Result, ScoredPoint, SharedSpaceGrantKey, Uuid};

pub(in crate::docs) fn doc_read_allowed(
	requester_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<SharedSpaceGrantKey>,
	owner_agent_id: &str,
	scope: &str,
) -> bool {
	if !allowed_scopes.iter().any(|s| s == scope) {
		return false;
	}
	if scope == "agent_private" {
		return owner_agent_id == requester_agent_id;
	}
	if owner_agent_id == requester_agent_id {
		return true;
	}

	shared_grants.contains(&SharedSpaceGrantKey {
		scope: scope.to_string(),
		space_owner_agent_id: owner_agent_id.to_string(),
	})
}

pub(in crate::docs) fn parse_scored_point_uuid_id(point: &ScoredPoint) -> Result<Uuid> {
	let id = point
		.id
		.as_ref()
		.ok_or_else(|| Error::Qdrant { message: "Qdrant returned item without id.".to_string() })?;

	match id.point_id_options.as_ref() {
		Some(PointIdOptions::Uuid(s)) => Uuid::parse_str(s.as_str())
			.map_err(|_| Error::Qdrant { message: "Qdrant returned invalid uuid id.".to_string() }),
		Some(other) => Err(Error::Qdrant {
			message: format!("Qdrant returned unsupported id type: {other:?}."),
		}),
		None => Err(Error::Qdrant { message: "Qdrant returned item with missing id.".to_string() }),
	}
}
