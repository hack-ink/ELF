pub use tokenizers::Tokenizer;
use unicode_segmentation::UnicodeSegmentation;

pub type TokenizerError = tokenizers::Error;

#[derive(Clone, Debug)]
pub struct ChunkingConfig {
	pub max_tokens: u32,
	pub overlap_tokens: u32,
}

#[derive(Clone, Debug)]
pub struct Chunk {
	pub chunk_index: i32,
	pub start_offset: usize,
	pub end_offset: usize,
	pub text: String,
}

pub fn load_tokenizer(repo: &str) -> Result<Tokenizer, TokenizerError> {
	Tokenizer::from_pretrained(repo, None)
}

pub fn split_text(text: &str, cfg: &ChunkingConfig, tokenizer: &Tokenizer) -> Vec<Chunk> {
	let sentences: Vec<(usize, &str)> = text.split_sentence_bound_indices().collect();
	let mut chunks = Vec::new();
	let mut current = String::new();
	let mut current_start = 0_usize;
	let mut last_end = 0_usize;
	let mut chunk_index = 0_i32;

	for (idx, sentence) in sentences {
		let candidate = format!("{}{}", current, sentence);
		let token_count = match tokenizer.encode(candidate.as_str(), false) {
			Ok(encoding) => encoding.len(),
			Err(err) => {
				tracing::error!(error = %err, "Tokenizer failed to encode sentence candidate.");

				0
			},
		};

		if token_count as u32 > cfg.max_tokens && !current.is_empty() {
			chunks.push(Chunk {
				chunk_index,
				start_offset: current_start,
				end_offset: last_end,
				text: current.clone(),
			});

			chunk_index += 1;

			let overlap = overlap_tail(&current, cfg.overlap_tokens, tokenizer);

			current_start = last_end.saturating_sub(overlap.len());
			current = overlap;
		}
		if current.is_empty() {
			current_start = idx;
		}

		current.push_str(sentence);

		last_end = idx + sentence.len();
	}

	if !current.is_empty() {
		chunks.push(Chunk {
			chunk_index,
			start_offset: current_start,
			end_offset: last_end,
			text: current,
		});
	}

	chunks
}

fn overlap_tail(text: &str, overlap_tokens: u32, tokenizer: &Tokenizer) -> String {
	if overlap_tokens == 0 {
		return String::new();
	}

	let encoding = match tokenizer.encode(text, false) {
		Ok(encoding) => encoding,
		Err(err) => {
			tracing::error!(error = %err, "Tokenizer failed to encode overlap tail.");

			return String::new();
		},
	};
	let tokens = encoding.get_ids();
	let start = tokens.len().saturating_sub(overlap_tokens as usize);
	let tail_ids = &tokens[start..];

	match tokenizer.decode(tail_ids, true) {
		Ok(decoded) => decoded,
		Err(err) => {
			tracing::error!(error = %err, "Tokenizer failed to decode overlap tail.");

			String::new()
		},
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn splits_into_chunks_with_overlap() {
		let cfg = ChunkingConfig { max_tokens: 10, overlap_tokens: 2 };
		let tokenizer = load_tokenizer("Qwen/Qwen3-Embedding-8B").unwrap();
		let chunks = split_text("One. Two. Three. Four.", &cfg, &tokenizer);

		assert!(!chunks.is_empty());
		assert!(chunks[0].text.contains("One"));
	}
}
