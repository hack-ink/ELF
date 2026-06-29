use serde::Deserialize;

/// Sentence-aware token chunking settings.
#[derive(Debug, Deserialize)]
pub struct Chunking {
	/// Whether chunking support is enabled.
	pub enabled: bool,
	/// Maximum tokens allowed in one chunk.
	pub max_tokens: u32,
	/// Number of tail tokens overlapped into the next chunk.
	pub overlap_tokens: u32,
	/// Hugging Face tokenizer repo used for token counting.
	pub tokenizer_repo: String,
}
