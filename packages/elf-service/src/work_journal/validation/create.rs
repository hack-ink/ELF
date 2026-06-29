use super::*;

pub(in crate::work_journal) fn validate_work_journal_create(
	cfg: &Config,
	req: &WorkJournalEntryCreateRequest,
) -> Result<ValidatedWorkJournalCreate> {
	validate_write_context(
		cfg,
		req.tenant_id.as_str(),
		req.project_id.as_str(),
		req.agent_id.as_str(),
		req.scope.as_str(),
	)?;
	validate_identifier(req.session_id.as_str(), "$.session_id")?;

	if req.body.trim().is_empty() {
		return Err(Error::InvalidRequest { message: "body must be non-empty.".to_string() });
	}
	if req.body.chars().count() > MAX_BODY_CHARS {
		return Err(Error::InvalidRequest {
			message: "body exceeds max journal size.".to_string(),
		});
	}

	let title = req.title.as_ref().map(|title| title.trim().to_string()).filter(|s| !s.is_empty());

	if let Some(title) = title.as_ref() {
		validate_natural_language(title.as_str(), "$.title")?;

		if writegate::contains_secrets(title.as_str()) {
			return Err(Error::InvalidRequest { message: "title contains secrets.".to_string() });
		}
	}

	let policy_result = writegate::apply_write_policy(req.body.as_str(), req.write_policy.as_ref())
		.map_err(|err| Error::InvalidRequest {
			message: format!("write_policy is invalid: {err:?}"),
		})?;
	let body = policy_result.transformed;

	if body.trim().is_empty() {
		return Err(Error::InvalidRequest { message: "body must be non-empty.".to_string() });
	}

	validate_natural_language(body.as_str(), "$.body")?;

	if writegate::contains_secrets(body.as_str()) {
		return Err(Error::InvalidRequest { message: "body contains secrets.".to_string() });
	}

	validate_text_list(&req.explicit_next_steps, "$.explicit_next_steps")?;
	validate_text_list(&req.inferred_next_steps, "$.inferred_next_steps")?;
	validate_text_list(&req.rejected_options, "$.rejected_options")?;

	let source_refs = validate_source_refs(&req.source_refs)?;
	let promotion_boundary = normalize_promotion_boundary(&req.promotion_boundary)?;
	let explicit_next_steps = serde_json::to_value(&req.explicit_next_steps).map_err(|err| {
		Error::InvalidRequest { message: format!("explicit_next_steps are invalid: {err}") }
	})?;
	let inferred_next_steps = serde_json::to_value(&req.inferred_next_steps).map_err(|err| {
		Error::InvalidRequest { message: format!("inferred_next_steps are invalid: {err}") }
	})?;
	let rejected_options = serde_json::to_value(&req.rejected_options).map_err(|err| {
		Error::InvalidRequest { message: format!("rejected_options are invalid: {err}") }
	})?;

	Ok(ValidatedWorkJournalCreate {
		entry_id: req.entry_id.unwrap_or_else(Uuid::new_v4),
		scope: req.scope.trim().to_string(),
		session_id: req.session_id.trim().to_string(),
		title,
		body,
		source_refs,
		explicit_next_steps,
		inferred_next_steps,
		rejected_options,
		promotion_boundary,
		redaction_audit: policy_result.audit,
	})
}

fn validate_text_list(values: &[String], path: &str) -> Result<()> {
	if values.len() > MAX_SIDE_LIST_ITEMS {
		return Err(Error::InvalidRequest { message: format!("{path} has too many items.") });
	}

	for (index, value) in values.iter().enumerate() {
		if value.trim().is_empty() {
			return Err(Error::InvalidRequest {
				message: format!("{path}[{index}] must be non-empty."),
			});
		}

		validate_natural_language(value.as_str(), format!("{path}[{index}]").as_str())?;

		if writegate::contains_secrets(value.as_str()) {
			return Err(Error::InvalidRequest {
				message: format!("{path}[{index}] contains secrets."),
			});
		}
	}

	Ok(())
}
