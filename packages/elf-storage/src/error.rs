#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error(transparent)]
	Sqlx(#[from] sqlx::Error),
	#[error("Invalid argument: {0}")]
	InvalidArgument(String),
	#[error("Not found: {0}")]
	NotFound(String),
	#[error("Conflict: {0}")]
	Conflict(String),
	#[error(transparent)]
	Qdrant(#[from] Box<qdrant_client::QdrantError>),
}
impl From<qdrant_client::QdrantError> for Error {
	fn from(err: qdrant_client::QdrantError) -> Self {
		Self::Qdrant(Box::new(err))
	}
}
