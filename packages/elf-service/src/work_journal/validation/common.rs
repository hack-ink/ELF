use crate::work_journal::validation::{Error, Map, Result, Value, english_gate, writegate};

pub(in crate::work_journal) fn validate_identifier(text: &str, field: &str) -> Result<()> {
	if text.trim().is_empty() || !english_gate::is_english_identifier(text.trim()) {
		return Err(Error::NonEnglishInput { field: field.to_string() });
	}

	Ok(())
}

pub(super) fn validate_natural_language(text: &str, field: &str) -> Result<()> {
	if !english_gate::is_english_natural_language(text) {
		return Err(Error::NonEnglishInput { field: field.to_string() });
	}

	Ok(())
}

pub(super) fn validate_json_strings(value: &Value, path: &str) -> Result<()> {
	match value {
		Value::String(text) => {
			validate_identifier(text.as_str(), path)?;

			if writegate::contains_secrets(text.as_str()) {
				return Err(Error::InvalidRequest { message: format!("{path} contains secrets.") });
			}
		},
		Value::Array(items) =>
			for (index, item) in items.iter().enumerate() {
				validate_json_strings(item, format!("{path}[{index}]").as_str())?;
			},
		Value::Object(map) =>
			for (key, item) in map {
				validate_identifier(key.as_str(), format!("{path}.{key}").as_str())?;
				validate_json_strings(item, format!("{path}.{key}").as_str())?;
			},
		Value::Null | Value::Bool(_) | Value::Number(_) => {},
	}

	Ok(())
}

pub(super) fn object_string<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
	map.get(key).and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty())
}
