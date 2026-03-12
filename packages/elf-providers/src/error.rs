/// Result alias for provider adapters.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors returned by provider adapters.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// HTTP transport or response decoding error from `reqwest`.
	#[error(transparent)]
	Reqwest(#[from] reqwest::Error),
	/// JSON encode or decode failure.
	#[error(transparent)]
	SerdeJson(#[from] serde_json::Error),
	/// Invalid HTTP header name in provider config.
	#[error(transparent)]
	InvalidHeaderName(#[from] reqwest::header::InvalidHeaderName),
	/// Invalid HTTP header value in provider config.
	#[error(transparent)]
	InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),
	/// Local provider configuration was invalid.
	#[error("{message}")]
	InvalidConfig {
		/// Human-readable configuration error.
		message: String,
	},
	/// Provider response shape was invalid.
	#[error("{message}")]
	InvalidResponse {
		/// Human-readable response validation error.
		message: String,
	},
}
