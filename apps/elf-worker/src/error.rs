pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("{0}")]
	Message(String),
	#[error("{0}")]
	Validation(String),
	#[error(transparent)]
	Sqlx(#[from] sqlx::Error),
	#[error(transparent)]
	Storage(#[from] elf_storage::Error),
	#[error(transparent)]
	Tokenizer(#[from] elf_chunking::TokenizerError),
	#[error(transparent)]
	SerdeJson(#[from] serde_json::Error),
	#[error(transparent)]
	Qdrant(#[from] Box<qdrant_client::QdrantError>),
}
impl From<qdrant_client::QdrantError> for Error {
	fn from(err: qdrant_client::QdrantError) -> Self {
		Self::Qdrant(Box::new(err))
	}
}
