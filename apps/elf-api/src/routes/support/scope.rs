use crate::routes::{
	ShareScope, StatusCode,
	support::errors::{self, ApiError},
};

pub(in super::super) fn parse_space(scope: &str) -> Result<ShareScope, ApiError> {
	match scope {
		"team_shared" | "project_shared" => Ok(ShareScope::ProjectShared),
		"org_shared" => Ok(ShareScope::OrgShared),
		_ => Err(errors::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Invalid space.".to_string(),
			Some(vec!["$.space".to_string()]),
		)),
	}
}

pub(in super::super) fn format_space(scope: ShareScope) -> &'static str {
	match scope {
		ShareScope::ProjectShared => "team_shared",
		ShareScope::OrgShared => "org_shared",
	}
}

pub(in super::super) fn format_scope(scope: &str) -> Result<&'static str, ApiError> {
	match scope {
		"project_shared" => Ok("team_shared"),
		"org_shared" => Ok("org_shared"),
		"agent_private" => Ok("agent_private"),
		_ => Err(errors::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Invalid space.".to_string(),
			Some(vec!["$.space".to_string()]),
		)),
	}
}
