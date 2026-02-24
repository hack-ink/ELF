pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Non-English input detected at {field}.")]
	NonEnglishInput { field: String },
	#[error("Invalid request: {message}")]
	InvalidRequest { message: String },
	#[error("Scope denied: {message}")]
	ScopeDenied { message: String },
	#[error("Not found: {message}")]
	NotFound { message: String },
	#[error("Conflict: {message}")]
	Conflict { message: String },
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

impl From<elf_storage::Error> for Error {
	fn from(err: elf_storage::Error) -> Self {
		match err {
			elf_storage::Error::Sqlx(inner) => Self::Storage { message: inner.to_string() },
			elf_storage::Error::InvalidArgument(message) => Self::InvalidRequest { message },
			elf_storage::Error::NotFound(message) => Self::NotFound { message },
			elf_storage::Error::Conflict(message) => Self::Conflict { message },
			elf_storage::Error::Qdrant(inner) => Self::Qdrant { message: inner.to_string() },
		}
	}
}
