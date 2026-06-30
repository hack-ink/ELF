use crate::docs::validation::{Error, Map, Result, Value};

pub(in crate::docs) fn extract_source_ref_string(
	source_ref: &Map<String, Value>,
	key: &str,
	path: &str,
) -> Result<String> {
	source_ref
		.get(key)
		.and_then(Value::as_str)
		.map(|text| text.trim().to_string())
		.filter(|text| !text.is_empty())
		.ok_or_else(|| Error::InvalidRequest { message: format!("{path} is required.") })
}

pub(in crate::docs::validation::source_ref) fn validate_optional_source_ref_string(
	source_ref: &Map<String, Value>,
	key: &str,
) -> Result<()> {
	let path = format!("$.source_ref[\"{key}\"]");

	validate_optional_source_ref_string_at(source_ref, key, path.as_str())
}

pub(in crate::docs::validation::source_ref) fn validate_optional_source_ref_string_at(
	source_ref: &Map<String, Value>,
	key: &str,
	path: &str,
) -> Result<()> {
	let Some(value) = source_ref.get(key) else {
		return Ok(());
	};

	value.as_str().map(str::trim).filter(|value| !value.is_empty()).ok_or_else(|| {
		Error::InvalidRequest { message: format!("{path} must be a non-empty string.") }
	})?;

	Ok(())
}
