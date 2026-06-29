use crate::routes::{
	OffsetDateTime, Rfc3339, StatusCode,
	support::errors::{self, ApiError},
};

pub(in super::super) fn parse_optional_rfc3339(
	raw: Option<&String>,
	path: &str,
) -> Result<Option<OffsetDateTime>, ApiError> {
	let Some(raw) = raw else {
		return Ok(None);
	};
	let raw = raw.trim();

	if raw.is_empty() {
		return Err(errors::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			format!("{path} must be non-empty."),
			Some(vec![path.to_string()]),
		));
	}

	OffsetDateTime::parse(raw, &Rfc3339).map(Some).map_err(|_| {
		errors::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			format!("{path} must be an RFC3339 datetime string."),
			Some(vec![path.to_string()]),
		)
	})
}
