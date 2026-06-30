use crate::docs::validation::{Error, Map, Result, Value, source_ref::strings};

pub(in crate::docs::validation::source_ref) fn validate_source_library_excerpt_locator(
	locator: &Value,
) -> Result<()> {
	let locator = locator.as_object().ok_or_else(|| Error::InvalidRequest {
		message: "$.source_ref[\"excerpt_locator\"] must be a JSON object.".to_string(),
	})?;
	let has_quote = locator.contains_key("quote");
	let has_position = locator.contains_key("position");

	if !has_quote && !has_position {
		return Err(Error::InvalidRequest {
			message: "$.source_ref[\"excerpt_locator\"] requires quote or position.".to_string(),
		});
	}

	if let Some(quote) = locator.get("quote") {
		validate_source_library_quote_locator(quote)?;
	}
	if let Some(position) = locator.get("position") {
		validate_source_library_position_locator(position)?;
	}

	Ok(())
}

fn validate_source_library_quote_locator(quote: &Value) -> Result<()> {
	let quote = quote.as_object().ok_or_else(|| Error::InvalidRequest {
		message: "$.source_ref[\"excerpt_locator\"][\"quote\"] must be a JSON object.".to_string(),
	})?;

	strings::extract_source_ref_string(
		quote,
		"exact",
		"$.source_ref[\"excerpt_locator\"][\"quote\"][\"exact\"]",
	)?;
	strings::validate_optional_source_ref_string_at(
		quote,
		"prefix",
		"$.source_ref[\"excerpt_locator\"][\"quote\"][\"prefix\"]",
	)?;
	strings::validate_optional_source_ref_string_at(
		quote,
		"suffix",
		"$.source_ref[\"excerpt_locator\"][\"quote\"][\"suffix\"]",
	)?;

	Ok(())
}

fn validate_source_library_position_locator(position: &Value) -> Result<()> {
	let position = position.as_object().ok_or_else(|| Error::InvalidRequest {
		message: "$.source_ref[\"excerpt_locator\"][\"position\"] must be a JSON object."
			.to_string(),
	})?;
	let start = source_ref_u64(
		position,
		"start",
		"$.source_ref[\"excerpt_locator\"][\"position\"][\"start\"]",
	)?;
	let end = source_ref_u64(
		position,
		"end",
		"$.source_ref[\"excerpt_locator\"][\"position\"][\"end\"]",
	)?;

	if start >= end {
		return Err(Error::InvalidRequest {
			message: "$.source_ref[\"excerpt_locator\"][\"position\"] start must be before end."
				.to_string(),
		});
	}

	Ok(())
}

fn source_ref_u64(source_ref: &Map<String, Value>, key: &str, path: &str) -> Result<u64> {
	source_ref
		.get(key)
		.and_then(Value::as_u64)
		.ok_or_else(|| Error::InvalidRequest { message: format!("{path} must be an integer.") })
}
