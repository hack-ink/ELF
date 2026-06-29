use super::super::*;

#[derive(Debug)]
pub(in super::super) struct ApiError {
	pub(in super::super) status: StatusCode,
	pub(in super::super) error_code: String,
	pub(in super::super) message: String,
	pub(in super::super) fields: Option<Vec<String>>,
}
impl ApiError {
	pub(in super::super) fn new(
		status: StatusCode,
		error_code: impl Into<String>,
		message: impl Into<String>,
		fields: Option<Vec<String>>,
	) -> Self {
		Self { status, error_code: error_code.into(), message: message.into(), fields }
	}
}

impl From<Error> for ApiError {
	fn from(err: Error) -> Self {
		match err {
			Error::NonEnglishInput { field } => json_error(
				StatusCode::UNPROCESSABLE_ENTITY,
				"NON_ENGLISH_INPUT",
				"Non-English input detected; upstream must canonicalize to English before calling ELF.",
				Some(vec![field]),
			),
			Error::InvalidRequest { message } =>
				json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", message, None),
			Error::ScopeDenied { message } =>
				json_error(StatusCode::FORBIDDEN, "SCOPE_DENIED", message, None),
			Error::NotFound { message } =>
				json_error(StatusCode::NOT_FOUND, "NOT_FOUND", message, None),
			Error::Conflict { message } =>
				json_error(StatusCode::CONFLICT, "CONFLICT", message, None),
			Error::Provider { message } => {
				let sanitized = sanitize_log_text(message.as_str());

				tracing::error!(error = %sanitized, "Provider error.");

				json_error(
					StatusCode::INTERNAL_SERVER_ERROR,
					"INTERNAL_ERROR",
					"Internal error.".to_string(),
					None,
				)
			},
			Error::Storage { message } => {
				let sanitized = sanitize_log_text(message.as_str());

				tracing::error!(error = %sanitized, "Storage error.");

				json_error(
					StatusCode::INTERNAL_SERVER_ERROR,
					"INTERNAL_ERROR",
					"Internal error.".to_string(),
					None,
				)
			},
			Error::Qdrant { message } => {
				let sanitized = sanitize_log_text(message.as_str());

				tracing::error!(error = %sanitized, "Qdrant error.");

				json_error(
					StatusCode::INTERNAL_SERVER_ERROR,
					"INTERNAL_ERROR",
					"Internal error.".to_string(),
					None,
				)
			},
		}
	}
}

impl IntoResponse for ApiError {
	fn into_response(self) -> Response {
		let body =
			ErrorBody { error_code: self.error_code, message: self.message, fields: self.fields };

		(self.status, Json(body)).into_response()
	}
}

pub(in super::super) fn json_error(
	status: StatusCode,
	code: &str,
	message: impl Into<String>,
	fields: Option<Vec<String>>,
) -> ApiError {
	ApiError::new(status, code, message, fields)
}

pub(in super::super) fn sanitize_log_text(text: &str) -> String {
	let mut parts = Vec::new();
	let mut redact_next = false;

	for raw in text.split_whitespace() {
		let mut word = raw.to_string();

		if redact_next {
			word = "[REDACTED]".to_string();
			redact_next = false;
		}
		if raw.eq_ignore_ascii_case("bearer") {
			redact_next = true;
		}

		let lowered = raw.to_ascii_lowercase();

		for key in ["api_key", "apikey", "password", "secret", "token"] {
			if lowered.contains(key) && (lowered.contains('=') || lowered.contains(':')) {
				let sep = if raw.contains('=') { '=' } else { ':' };
				let prefix = match raw.split(sep).next() {
					Some(prefix) => prefix,
					None => raw,
				};

				word = format!("{prefix}{sep}[REDACTED]");

				break;
			}
		}

		parts.push(word);
	}

	let mut out = parts.join(" ");

	if out.chars().count() > MAX_ERROR_LOG_CHARS {
		out = out.chars().take(MAX_ERROR_LOG_CHARS).collect();

		out.push_str("...");
	}

	out
}
