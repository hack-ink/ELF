/// Storage-layer errors returned by Postgres and Qdrant helpers.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// A SQLx query or connection operation failed.
	#[error(transparent)]
	Sqlx(#[from] sqlx::Error),
	/// The caller supplied an invalid storage argument.
	#[error("Invalid argument: {0}")]
	InvalidArgument(String),
	/// The requested storage record does not exist.
	#[error("Not found: {0}")]
	NotFound(String),
	/// The requested storage mutation conflicts with existing state.
	#[error("Conflict: {0}")]
	Conflict(String),
	/// A Qdrant client operation failed.
	#[error(transparent)]
	Qdrant(#[from] Box<qdrant_client::QdrantError>),
}
impl From<qdrant_client::QdrantError> for Error {
	fn from(err: qdrant_client::QdrantError) -> Self {
		Self::Qdrant(Box::new(err))
	}
}
