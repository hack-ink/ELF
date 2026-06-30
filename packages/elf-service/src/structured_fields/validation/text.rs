use crate::{Error, Result};
use elf_domain::english_gate;

const MAX_ITEM_CHARS: usize = 1_000;

pub(super) fn validate_text_field(value: &str, label: &str) -> Result<()> {
	let trimmed = value.trim();

	if trimmed.is_empty() {
		return Err(Error::InvalidRequest { message: format!("{label} must not be empty.") });
	}
	if trimmed.chars().count() > MAX_ITEM_CHARS {
		return Err(Error::InvalidRequest {
			message: format!("{label} must be at most {MAX_ITEM_CHARS} characters."),
		});
	}
	if !english_gate::is_english_natural_language(trimmed) {
		return Err(Error::NonEnglishInput { field: label.to_string() });
	}

	Ok(())
}

pub(super) fn validate_required_text_field(value: Option<&String>, label: &str) -> Result<()> {
	let Some(value) = value else {
		return Err(Error::InvalidRequest { message: format!("{label} is required.") });
	};

	validate_text_field(value, label)
}
