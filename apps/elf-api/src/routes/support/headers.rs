use super::{
	super::*,
	errors::{ApiError, json_error},
};

#[derive(Clone, Debug)]
pub(in super::super) struct RequestContext {
	pub(in super::super) tenant_id: String,
	pub(in super::super) project_id: String,
	pub(in super::super) agent_id: String,
}
impl RequestContext {
	pub(in super::super) fn from_headers(headers: &HeaderMap) -> Result<Self, ApiError> {
		let tenant_id = required_header(headers, HEADER_TENANT_ID)?;
		let project_id = required_header(headers, HEADER_PROJECT_ID)?;
		let agent_id = required_header(headers, HEADER_AGENT_ID)?;

		Ok(Self { tenant_id, project_id, agent_id })
	}
}

pub(in super::super) fn required_header(
	headers: &HeaderMap,
	name: &'static str,
) -> Result<String, ApiError> {
	let raw = headers.get(name).ok_or_else(|| {
		json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			format!("{name} header is required."),
			Some(vec![format!("$.headers.{name}")]),
		)
	})?;
	let value = raw.to_str().map_err(|_| {
		json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			format!("{name} header must be a valid string."),
			Some(vec![format!("$.headers.{name}")]),
		)
	})?;
	let trimmed = value.trim();

	if trimmed.is_empty() {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			format!("{name} header must be non-empty."),
			Some(vec![format!("$.headers.{name}")]),
		));
	}
	if trimmed.chars().count() > MAX_CONTEXT_HEADER_CHARS {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			format!("{name} header is too long."),
			Some(vec![format!("$.headers.{name}")]),
		));
	}
	if !english_gate::is_english_identifier(trimmed) {
		return Err(json_error(
			StatusCode::UNPROCESSABLE_ENTITY,
			"NON_ENGLISH_INPUT",
			"Non-English input detected; upstream must canonicalize to English before calling ELF."
				.to_string(),
			Some(vec![format!("$.headers.{name}")]),
		));
	}

	Ok(trimmed.to_string())
}

pub(in super::super) fn required_read_profile(headers: &HeaderMap) -> Result<String, ApiError> {
	required_header(headers, HEADER_READ_PROFILE)
}
