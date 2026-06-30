use crate::docs::validation::{
	Error, Map, OffsetDateTime, Result, Rfc3339, SOURCE_LIBRARY_FIELD_KEYS, SOURCE_LIBRARY_KINDS,
	SOURCE_LIBRARY_TRUST_LABELS, Value,
	source_ref::{locators, strings},
};

pub(in crate::docs) fn validate_source_library_metadata(
	source_doc_type: &str,
	source_ref: &Map<String, Value>,
) -> Result<()> {
	if !source_library_metadata_present(source_ref) {
		return Ok(());
	}

	let source_kind = strings::extract_source_ref_string(
		source_ref,
		"source_kind",
		"$.source_ref[\"source_kind\"]",
	)?;

	if !SOURCE_LIBRARY_KINDS.contains(&source_kind.as_str()) {
		return Err(Error::InvalidRequest {
			message: format!(
				"$.source_ref[\"source_kind\"] must be one of: {}.",
				SOURCE_LIBRARY_KINDS.join("|")
			),
		});
	}

	validate_source_kind_doc_type(source_kind.as_str(), source_doc_type)?;

	strings::extract_source_ref_string(
		source_ref,
		"canonical_uri",
		"$.source_ref[\"canonical_uri\"]",
	)?;

	validate_source_ref_rfc3339(source_ref, "captured_at")?;

	if source_ref.contains_key("source_created_at") {
		validate_source_ref_rfc3339(source_ref, "source_created_at")?;
	}

	let trust_label = strings::extract_source_ref_string(
		source_ref,
		"trust_label",
		"$.source_ref[\"trust_label\"]",
	)?;

	if !SOURCE_LIBRARY_TRUST_LABELS.contains(&trust_label.as_str()) {
		return Err(Error::InvalidRequest {
			message: format!(
				"$.source_ref[\"trust_label\"] must be one of: {}.",
				SOURCE_LIBRARY_TRUST_LABELS.join("|")
			),
		});
	}

	strings::validate_optional_source_ref_string(source_ref, "author")?;
	strings::validate_optional_source_ref_string(source_ref, "handle")?;
	strings::validate_optional_source_ref_string(source_ref, "source_content_hash")?;

	if let Some(locator) = source_ref.get("excerpt_locator") {
		locators::validate_source_library_excerpt_locator(locator)?;
	}

	Ok(())
}

fn source_library_metadata_present(source_ref: &Map<String, Value>) -> bool {
	SOURCE_LIBRARY_FIELD_KEYS.iter().any(|key| source_ref.contains_key(*key))
}

fn validate_source_kind_doc_type(source_kind: &str, source_doc_type: &str) -> Result<()> {
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

fn validate_source_ref_rfc3339(source_ref: &Map<String, Value>, key: &str) -> Result<()> {
	let path = format!("$.source_ref[\"{key}\"]");
	let value = strings::extract_source_ref_string(source_ref, key, path.as_str())?;

	OffsetDateTime::parse(value.as_str(), &Rfc3339).map_err(|_| Error::InvalidRequest {
		message: format!("{path} must be an RFC3339 datetime string."),
	})?;

	Ok(())
}
