use crate::docs::validation::{Error, Map, Result, Value, source_ref::strings};

pub(in crate::docs) fn validate_doc_source_ref_requirements(
	source_doc_type: &str,
	source_ref: &Map<String, Value>,
) -> Result<()> {
	match source_doc_type {
		"chat" => {
			strings::extract_source_ref_string(
				source_ref,
				"thread_id",
				"$.source_ref[\"thread_id\"]",
			)?;
			strings::extract_source_ref_string(source_ref, "role", "$.source_ref[\"role\"]")?;
		},
		"search" => {
			strings::extract_source_ref_string(source_ref, "query", "$.source_ref[\"query\"]")?;
			strings::extract_source_ref_string(source_ref, "url", "$.source_ref[\"url\"]")?;
			strings::extract_source_ref_string(source_ref, "domain", "$.source_ref[\"domain\"]")?;
		},
		"dev" => {
			strings::extract_source_ref_string(source_ref, "repo", "$.source_ref[\"repo\"]")?;

			validate_dev_revision_ref(source_ref)?;
		},
		"knowledge" => {},
		_ => unreachable!(),
	}

	Ok(())
}

fn validate_dev_revision_ref(source_ref: &Map<String, Value>) -> Result<()> {
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

	Ok(())
}
