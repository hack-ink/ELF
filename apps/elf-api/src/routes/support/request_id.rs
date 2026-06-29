use crate::routes::{
	Body, CONTENT_LENGTH, CONTENT_TYPE, HEADER_REQUEST_ID, HeaderMap, Response, StatusCode, Uuid,
	Value, body,
	support::errors::{self, ApiError},
};

pub(in super::super) fn parse_request_id_from_headers(
	headers: &HeaderMap,
) -> Result<Uuid, ApiError> {
	if let Some(raw) = headers.get(HEADER_REQUEST_ID) {
		let raw = raw.to_str().map_err(|_| {
			errors::json_error(
				StatusCode::BAD_REQUEST,
				"INVALID_REQUEST",
				format!("{HEADER_REQUEST_ID} header must be a valid string."),
				Some(vec![format!("$.headers.{HEADER_REQUEST_ID}")]),
			)
		})?;
		let trimmed = raw.trim();

		if trimmed.is_empty() {
			return Err(errors::json_error(
				StatusCode::BAD_REQUEST,
				"INVALID_REQUEST",
				format!("{HEADER_REQUEST_ID} header must be non-empty."),
				Some(vec![format!("$.headers.{HEADER_REQUEST_ID}")]),
			));
		}

		Uuid::parse_str(trimmed).map_err(|_| {
			errors::json_error(
				StatusCode::BAD_REQUEST,
				"INVALID_REQUEST",
				format!("{HEADER_REQUEST_ID} header must be a valid UUID."),
				Some(vec![format!("$.headers.{HEADER_REQUEST_ID}")]),
			)
		})
	} else {
		Ok(Uuid::new_v4())
	}
}

pub(in super::super) fn inject_request_id_into_json_body(
	body: &[u8],
	request_id: &Uuid,
) -> Option<Vec<u8>> {
	let mut response_body: Value = serde_json::from_slice(body).ok()?;
	let object = response_body.as_object_mut()?;

	object.insert("request_id".to_string(), Value::String(request_id.to_string()));

	serde_json::to_vec(&response_body).ok()
}

pub(in super::super) async fn with_request_id(response: Response, request_id: Uuid) -> Response {
	let (mut parts, body) = response.into_parts();

	parts.headers.insert(
		HEADER_REQUEST_ID,
		request_id.to_string().parse().expect("request_id is valid uuid string"),
	);

	let is_json_response = parts
		.headers
		.get(CONTENT_TYPE)
		.and_then(|value| value.to_str().ok())
		.map(|content_type| content_type.starts_with("application/json"))
		.unwrap_or(false);

	if !is_json_response {
		return Response::from_parts(parts, body);
	}

	let body_bytes = match body::to_bytes(body, usize::MAX).await {
		Ok(bytes) => bytes,
		Err(_) => return Response::from_parts(parts, Body::empty()),
	};

	if let Some(response_body) = inject_request_id_into_json_body(&body_bytes, &request_id) {
		parts.headers.remove(CONTENT_LENGTH);

		Response::from_parts(parts, Body::from(response_body))
	} else {
		Response::from_parts(parts, Body::from(body_bytes))
	}
}
