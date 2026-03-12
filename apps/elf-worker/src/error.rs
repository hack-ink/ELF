/// Worker-app result type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors returned by the ELF worker app.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// Generic worker failure with a human-readable message.
	#[error("{0}")]
	Message(String),
	/// Validation failure while preparing worker operations.
	#[error("{0}")]
	Validation(String),
	/// SQLx query or connection failure.
	#[error(transparent)]
	Sqlx(#[from] sqlx::Error),
	/// Storage-layer failure.
	#[error(transparent)]
	Storage(#[from] elf_storage::Error),
	/// Tokenizer or chunking failure.
	#[error(transparent)]
	Tokenizer(#[from] elf_chunking::Error),
	/// JSON serialization or deserialization failure.
	#[error(transparent)]
	SerdeJson(#[from] serde_json::Error),
	/// Qdrant client failure.
	#[error(transparent)]
	Qdrant(#[from] Box<qdrant_client::QdrantError>),
}
impl From<qdrant_client::QdrantError> for Error {
	fn from(err: qdrant_client::QdrantError) -> Self {
		Self::Qdrant(Box::new(err))
	}
}
