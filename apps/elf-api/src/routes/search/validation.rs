use crate::routes::{
	self, ApiError, JsonRejection, MAX_CANDIDATE_K, MAX_NOTE_IDS_PER_DETAILS, MAX_QUERY_CHARS,
	MAX_TOP_K, QueryRejection, SearchCreateRequest, SearchDetailsBody, StatusCode,
};

pub(super) fn invalid_json_payload(err: JsonRejection) -> ApiError {
	tracing::warn!(error = %err, "Invalid request payload.");

	routes::json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
}

pub(super) fn invalid_query_parameters(err: QueryRejection) -> ApiError {
	tracing::warn!(error = %err, "Invalid query parameters.");

	routes::json_error(
		StatusCode::BAD_REQUEST,
		"INVALID_REQUEST",
		"Invalid query parameters.",
		None,
	)
}

pub(super) fn validate_search_create_payload(
	payload: &SearchCreateRequest,
	default_top_k: u32,
	default_candidate_k: u32,
) -> Result<(), ApiError> {
	validate_search_limits(
		payload.query.as_str(),
		payload.top_k,
		payload.candidate_k,
		default_top_k,
		default_candidate_k,
	)?;

	if payload.ranking.is_some() {
		return Err(routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Ranking overrides are only supported on admin endpoints.",
			None,
		));
	}

	Ok(())
}

pub(super) fn validate_search_raw_payload(
	payload: &SearchCreateRequest,
	default_top_k: u32,
	default_candidate_k: u32,
) -> Result<(), ApiError> {
	validate_search_limits(
		payload.query.as_str(),
		payload.top_k,
		payload.candidate_k,
		default_top_k,
		default_candidate_k,
	)
}

pub(super) fn validate_search_details_payload(payload: &SearchDetailsBody) -> Result<(), ApiError> {
	if payload.note_ids.len() > MAX_NOTE_IDS_PER_DETAILS {
		return Err(routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"note_ids list is too large.",
			Some(vec!["$.note_ids".to_string()]),
		));
	}

	Ok(())
}

fn validate_search_limits(
	query: &str,
	top_k: Option<u32>,
	candidate_k: Option<u32>,
	default_top_k: u32,
	default_candidate_k: u32,
) -> Result<(), ApiError> {
	if query.chars().count() > MAX_QUERY_CHARS {
		return Err(routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Query is too long.",
			Some(vec!["$.query".to_string()]),
		));
	}
	if top_k.unwrap_or(default_top_k) > MAX_TOP_K {
		return Err(routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"top_k is too large.",
			Some(vec!["$.top_k".to_string()]),
		));
	}
	if candidate_k.unwrap_or(default_candidate_k) > MAX_CANDIDATE_K {
		return Err(routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"candidate_k is too large.",
			Some(vec!["$.candidate_k".to_string()]),
		));
	}

	Ok(())
}
