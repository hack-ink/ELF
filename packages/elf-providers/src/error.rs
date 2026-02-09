pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error(transparent)]
	Reqwest(#[from] reqwest::Error),
	#[error(transparent)]
	SerdeJson(#[from] serde_json::Error),
	#[error(transparent)]
	InvalidHeaderName(#[from] reqwest::header::InvalidHeaderName),
	#[error(transparent)]
	InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),
	#[error("{message}")]
	InvalidConfig { message: String },
	#[error("{message}")]
	InvalidResponse { message: String },
}
