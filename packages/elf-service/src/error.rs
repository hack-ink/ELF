pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Non-English input detected at {field}.")]
	NonEnglishInput { field: String },
	#[error("Invalid request: {message}")]
	InvalidRequest { message: String },
	#[error("Scope denied: {message}")]
	ScopeDenied { message: String },
	#[error("Provider error: {message}")]
	Provider { message: String },
	#[error("Storage error: {message}")]
	Storage { message: String },
	#[error("Qdrant error: {message}")]
	Qdrant { message: String },
}
impl From<sqlx::Error> for Error {
	fn from(err: sqlx::Error) -> Self {
		Self::Storage { message: err.to_string() }
	}
}
