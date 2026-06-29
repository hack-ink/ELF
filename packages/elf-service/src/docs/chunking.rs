use super::*;

pub(super) fn load_tokenizer(cfg: &Config) -> Result<Tokenizer> {
	let tokenizer_repo = cfg.chunking.tokenizer_repo.trim();

	if tokenizer_repo.is_empty() {
		return Err(Error::InvalidRequest {
			message: "chunking.tokenizer_repo must be set.".to_string(),
		});
	}

	elf_chunking::load_tokenizer(tokenizer_repo).map_err(|err| Error::InvalidRequest {
		message: format!("failed to load tokenizer: {err}"),
	})
}

pub(super) fn split_tokens_by_offsets(
	text: &str,
	profile_max_tokens: usize,
	profile_overlap_tokens: usize,
	max_chunks: usize,
	tokenizer: &Tokenizer,
) -> Result<Vec<ByteChunk>> {
	if profile_max_tokens == 0 {
		return Err(Error::InvalidRequest {
			message: "max_tokens must be greater than zero.".to_string(),
		});
	}
	if profile_overlap_tokens >= profile_max_tokens {
		return Err(Error::InvalidRequest {
			message: "overlap_tokens must be less than max_tokens.".to_string(),
		});
	}

	let encoding = tokenizer.encode(text, false).map_err(|err| Error::InvalidRequest {
		message: format!("failed to tokenize content: {err}"),
	})?;
	let offsets = encoding.get_offsets();
	let mut chunks = Vec::new();

	if offsets.is_empty() {
		return Ok(Vec::new());
	}

	let mut chunk_start_token = 0_usize;

	while chunk_start_token < offsets.len() {
		let chunk_end_token = (chunk_start_token + profile_max_tokens).min(offsets.len());
		let (start_offset, end_offset) = {
			let (start, _) = offsets[chunk_start_token];
			let (_, end) = offsets[chunk_end_token.saturating_sub(1)];

			(start, end)
		};
		let chunk_text =
			text.get(start_offset..end_offset).ok_or_else(|| Error::InvalidRequest {
				message: "computed chunk offset is invalid UTF-8 boundary.".to_string(),
			})?;

		chunks.push(ByteChunk {
			chunk_id: Uuid::new_v4(),
			start_offset,
			end_offset,
			text: chunk_text.to_string(),
		});

		if chunk_end_token >= offsets.len() {
			break;
		}
		if chunks.len() >= max_chunks {
			return Err(Error::InvalidRequest {
				message: "doc exceeds max_chunks_per_doc.".to_string(),
			});
		}

		chunk_start_token = chunk_end_token.saturating_sub(profile_overlap_tokens);
	}

	Ok(chunks)
}
