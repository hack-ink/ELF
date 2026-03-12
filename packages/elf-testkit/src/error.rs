/// Result alias for ELF testkit helpers.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors returned by ELF integration-test helpers.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// A helper-specific failure message.
	#[error("{0}")]
	Message(String),

	/// SQLx returned an error while creating or cleaning test databases.
	#[error(transparent)]
	Sqlx(#[from] sqlx::Error),

	/// Qdrant returned an error while managing test collections.
	#[error(transparent)]
	Qdrant(#[from] Box<qdrant_client::QdrantError>),
}
impl From<qdrant_client::QdrantError> for Error {
	fn from(err: qdrant_client::QdrantError) -> Self {
		Self::Qdrant(Box::new(err))
	}
}
