use crate::{Error, Result};
use elf_config::Config;

pub(crate) fn embedding_version(cfg: &Config) -> String {
	format!(
		"{}:{}:{}",
		cfg.providers.embedding.provider_id,
		cfg.providers.embedding.model,
		cfg.storage.qdrant.vector_dim
	)
}

pub(crate) fn vector_to_pg(vec: &[f32]) -> String {
	let mut out = String::with_capacity(vec.len() * 8);

	out.push('[');

	for (i, value) in vec.iter().enumerate() {
		if i > 0 {
			out.push(',');
		}

		out.push_str(&value.to_string());
	}

	out.push(']');

	out
}

pub(crate) fn parse_pg_vector(text: &str) -> Result<Vec<f32>> {
	let trimmed = text.trim();
	let without_brackets =
		trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')).ok_or_else(|| {
			Error::InvalidRequest { message: "Vector text is not bracketed.".to_string() }
		})?;

	if without_brackets.trim().is_empty() {
		return Ok(Vec::new());
	}

	let mut vec = Vec::new();

	for part in without_brackets.split(',') {
		let value: f32 = part.trim().parse().map_err(|_| Error::InvalidRequest {
			message: "Vector text contains a non-numeric value.".to_string(),
		})?;

		vec.push(value);
	}

	Ok(vec)
}
