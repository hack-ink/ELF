pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("{0}")]
	Message(String),

	#[error(transparent)]
	Sqlx(#[from] sqlx::Error),

	#[error(transparent)]
	Qdrant(#[from] Box<qdrant_client::QdrantError>),
}
impl From<qdrant_client::QdrantError> for Error {
	fn from(err: qdrant_client::QdrantError) -> Self {
		Self::Qdrant(Box::new(err))
	}
}
