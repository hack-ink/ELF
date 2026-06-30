use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::search::filter::{
	expr::FilterField,
	parser::{FilterParseError, MAX_STRING_BYTES},
	value::FilterValue,
};

pub(in crate::search::filter) fn parse_value(
	field: &FilterField,
	raw: &Value,
	path: &str,
) -> Result<FilterValue, FilterParseError> {
	match field {
		FilterField::Type | FilterField::Key | FilterField::Scope | FilterField::AgentId =>
			match raw {
				Value::String(_) | Value::Null if matches!(field, FilterField::Key) => {
					if raw.is_null() {
						Ok(FilterValue::Null)
					} else {
						parse_string(path, raw).map(FilterValue::String)
					}
				},
				_ => parse_string(path, raw).map(FilterValue::String),
			},
		FilterField::Importance | FilterField::Confidence | FilterField::HitCount => {
			let value = raw.as_f64().ok_or_else(|| FilterParseError {
				path: path.to_string(),
				message: "numeric value expected.".to_string(),
			})?;

			Ok(FilterValue::Number(value))
		},
		FilterField::UpdatedAt =>
			OffsetDateTime::parse(parse_string(path, raw)?.as_str(), &Rfc3339)
				.map(FilterValue::DateTime)
				.map_err(|_| FilterParseError {
					path: path.to_string(),
					message: "datetime value must be RFC3339.".to_string(),
				}),
		FilterField::ExpiresAt | FilterField::LastHitAt =>
			if raw.is_null() {
				Ok(FilterValue::Null)
			} else {
				OffsetDateTime::parse(parse_string(path, raw)?.as_str(), &Rfc3339)
					.map(FilterValue::DateTime)
					.map_err(|_| FilterParseError {
						path: path.to_string(),
						message: "datetime value must be RFC3339.".to_string(),
					})
			},
	}
}

fn parse_string(path: &str, raw: &Value) -> Result<String, FilterParseError> {
	let value = raw.as_str().ok_or_else(|| FilterParseError {
		path: path.to_string(),
		message: "string value expected.".to_string(),
	})?;

	if value.len() > MAX_STRING_BYTES {
		return Err(FilterParseError {
			path: path.to_string(),
			message: format!("string value exceeds maximum bytes ({}).", MAX_STRING_BYTES),
		});
	}

	Ok(value.to_string())
}
