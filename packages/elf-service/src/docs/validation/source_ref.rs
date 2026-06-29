use crate::docs::validation::{
	Error, Map, OffsetDateTime, Result, Rfc3339, SOURCE_LIBRARY_FIELD_KEYS, SOURCE_LIBRARY_KINDS,
	SOURCE_LIBRARY_TRUST_LABELS, Value,
};

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

pub(in crate::docs) fn validate_doc_source_ref_requirements(
	source_doc_type: &str,
	source_ref: &Map<String, Value>,
) -> Result<()> {
	match source_doc_type {
		"chat" => {
			extract_source_ref_string(source_ref, "thread_id", "$.source_ref[\"thread_id\"]")?;
			extract_source_ref_string(source_ref, "role", "$.source_ref[\"role\"]")?;
		},
		"search" => {
			extract_source_ref_string(source_ref, "query", "$.source_ref[\"query\"]")?;
			extract_source_ref_string(source_ref, "url", "$.source_ref[\"url\"]")?;
			extract_source_ref_string(source_ref, "domain", "$.source_ref[\"domain\"]")?;
		},
		"dev" => {
			extract_source_ref_string(source_ref, "repo", "$.source_ref[\"repo\"]")?;

			let commit_sha_present = source_ref
				.get("commit_sha")
				.and_then(Value::as_str)
				.is_some_and(|value| !value.trim().is_empty());
			let pr_number_present = source_ref
				.get("pr_number")
				.is_some_and(|value| value.as_i64().is_some() || value.as_u64().is_some());
			let issue_number_present = source_ref
				.get("issue_number")
				.is_some_and(|value| value.as_i64().is_some() || value.as_u64().is_some());
			let present_count =
				commit_sha_present as u8 + pr_number_present as u8 + issue_number_present as u8;

			if present_count != 1 {
				return Err(Error::InvalidRequest {
					message:
						"For doc_type=dev, exactly one of commit_sha, pr_number, or issue_number is required."
							.to_string(),
				});
			}
		},
		"knowledge" => {},
		_ => unreachable!(),
	}

	Ok(())
}

pub(in crate::docs) fn validate_source_library_metadata(
	source_doc_type: &str,
	source_ref: &Map<String, Value>,
) -> Result<()> {
	if !source_library_metadata_present(source_ref) {
		return Ok(());
	}

	let source_kind =
		extract_source_ref_string(source_ref, "source_kind", "$.source_ref[\"source_kind\"]")?;

	if !SOURCE_LIBRARY_KINDS.contains(&source_kind.as_str()) {
		return Err(Error::InvalidRequest {
			message: format!(
				"$.source_ref[\"source_kind\"] must be one of: {}.",
				SOURCE_LIBRARY_KINDS.join("|")
			),
		});
	}

	validate_source_kind_doc_type(source_kind.as_str(), source_doc_type)?;
	extract_source_ref_string(source_ref, "canonical_uri", "$.source_ref[\"canonical_uri\"]")?;
	validate_source_ref_rfc3339(source_ref, "captured_at")?;

	if source_ref.contains_key("source_created_at") {
		validate_source_ref_rfc3339(source_ref, "source_created_at")?;
	}

	let trust_label =
		extract_source_ref_string(source_ref, "trust_label", "$.source_ref[\"trust_label\"]")?;

	if !SOURCE_LIBRARY_TRUST_LABELS.contains(&trust_label.as_str()) {
		return Err(Error::InvalidRequest {
			message: format!(
				"$.source_ref[\"trust_label\"] must be one of: {}.",
				SOURCE_LIBRARY_TRUST_LABELS.join("|")
			),
		});
	}

	validate_optional_source_ref_string(source_ref, "author")?;
	validate_optional_source_ref_string(source_ref, "handle")?;
	validate_optional_source_ref_string(source_ref, "source_content_hash")?;

	if let Some(locator) = source_ref.get("excerpt_locator") {
		validate_source_library_excerpt_locator(locator)?;
	}

	Ok(())
}

pub(in crate::docs) fn source_library_metadata_present(source_ref: &Map<String, Value>) -> bool {
	SOURCE_LIBRARY_FIELD_KEYS.iter().any(|key| source_ref.contains_key(*key))
}

pub(in crate::docs) fn validate_source_kind_doc_type(
	source_kind: &str,
	source_doc_type: &str,
) -> Result<()> {
	let expected_doc_type = match source_kind {
		"social_thread" | "chat_excerpt" => Some("chat"),
		"repo_file" => Some("dev"),
		_ => None,
	};

	if let Some(expected_doc_type) = expected_doc_type
		&& source_doc_type != expected_doc_type
	{
		return Err(Error::InvalidRequest {
			message: format!(
				"$.source_ref[\"source_kind\"]={source_kind} requires doc_type={expected_doc_type}."
			),
		});
	}

	Ok(())
}

pub(in crate::docs) fn validate_source_ref_rfc3339(
	source_ref: &Map<String, Value>,
	key: &str,
) -> Result<()> {
	let path = format!("$.source_ref[\"{key}\"]");
	let value = extract_source_ref_string(source_ref, key, path.as_str())?;

	OffsetDateTime::parse(value.as_str(), &Rfc3339).map_err(|_| Error::InvalidRequest {
		message: format!("{path} must be an RFC3339 datetime string."),
	})?;

	Ok(())
}

pub(in crate::docs) fn validate_optional_source_ref_string(
	source_ref: &Map<String, Value>,
	key: &str,
) -> Result<()> {
	let path = format!("$.source_ref[\"{key}\"]");

	validate_optional_source_ref_string_at(source_ref, key, path.as_str())
}

pub(in crate::docs) fn validate_optional_source_ref_string_at(
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

pub(in crate::docs) fn validate_source_library_excerpt_locator(locator: &Value) -> Result<()> {
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

pub(in crate::docs) fn validate_source_library_quote_locator(quote: &Value) -> Result<()> {
	let quote = quote.as_object().ok_or_else(|| Error::InvalidRequest {
		message: "$.source_ref[\"excerpt_locator\"][\"quote\"] must be a JSON object.".to_string(),
	})?;

	extract_source_ref_string(
		quote,
		"exact",
		"$.source_ref[\"excerpt_locator\"][\"quote\"][\"exact\"]",
	)?;
	validate_optional_source_ref_string_at(
		quote,
		"prefix",
		"$.source_ref[\"excerpt_locator\"][\"quote\"][\"prefix\"]",
	)?;
	validate_optional_source_ref_string_at(
		quote,
		"suffix",
		"$.source_ref[\"excerpt_locator\"][\"quote\"][\"suffix\"]",
	)?;

	Ok(())
}

pub(in crate::docs) fn validate_source_library_position_locator(position: &Value) -> Result<()> {
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

pub(in crate::docs) fn source_ref_u64(
	source_ref: &Map<String, Value>,
	key: &str,
	path: &str,
) -> Result<u64> {
	source_ref
		.get(key)
		.and_then(Value::as_u64)
		.ok_or_else(|| Error::InvalidRequest { message: format!("{path} must be an integer.") })
}
