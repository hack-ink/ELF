/// Service-layer result type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors returned by ELF service APIs.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// The request contained non-English input in the named field path.
	#[error("Non-English input detected at {field}.")]
	NonEnglishInput {
		/// Field path that failed the English gate.
		field: String,
	},
	/// The request payload was invalid.
	#[error("Invalid request: {message}")]
	InvalidRequest {
		/// Human-readable validation failure.
		message: String,
	},
	/// The caller is not allowed to act on the requested scope.
	#[error("Scope denied: {message}")]
	ScopeDenied {
		/// Human-readable access failure.
		message: String,
	},
	/// The requested service resource could not be found.
	#[error("Not found: {message}")]
	NotFound {
		/// Human-readable lookup failure.
		message: String,
	},
	/// The requested mutation conflicts with existing state.
	#[error("Conflict: {message}")]
	Conflict {
		/// Human-readable conflict reason.
		message: String,
	},
	/// An external model or provider returned an error.
	#[error("Provider error: {message}")]
	Provider {
		/// Human-readable provider failure.
		message: String,
	},
	/// Postgres or other storage work failed.
	#[error("Storage error: {message}")]
	Storage {
		/// Human-readable storage failure.
		message: String,
	},
	/// Qdrant vector-store work failed.
	#[error("Qdrant error: {message}")]
	Qdrant {
		/// Human-readable Qdrant failure.
		message: String,
	},
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
