use crate::docs::validation::{
	DEFAULT_DOC_MAX_BYTES, DocType, DocsPutRequest, Error, OffsetDateTime, Result, Rfc3339,
	ValidatedDocsPut, english_gate, non_english,
	source_ref::{self},
	writegate,
};

pub(in crate::docs) fn validate_docs_put(req: &DocsPutRequest) -> Result<ValidatedDocsPut> {
	if req.content.trim().is_empty() {
		return Err(Error::InvalidRequest { message: "content must be non-empty.".to_string() });
	}
	if req.scope.trim().is_empty() {
		return Err(Error::InvalidRequest { message: "scope must be non-empty.".to_string() });
	}
	if !matches!(req.scope.as_str(), "agent_private" | "project_shared" | "org_shared") {
		return Err(Error::InvalidRequest { message: "Unknown scope.".to_string() });
	}

	let source_ref = req.source_ref.as_object().ok_or_else(|| Error::InvalidRequest {
		message: "source_ref must be a JSON object.".to_string(),
	})?;
	let source_ref_doc_type = source_ref::extract_source_ref_string(
		source_ref,
		"doc_type",
		"$.source_ref[\"doc_type\"]",
	)?;
	let source_ref_doc_type = DocType::parse(&source_ref_doc_type)?;
	let source_ref_schema =
		source_ref::extract_source_ref_string(source_ref, "schema", "$.source_ref[\"schema\"]")?;

	if source_ref_schema != "doc_source_ref/v1" {
		return Err(Error::InvalidRequest {
			message: "source_ref.schema must be 'doc_source_ref/v1'.".to_string(),
		});
	}

	let ts = source_ref::extract_source_ref_string(source_ref, "ts", "$.source_ref[\"ts\"]")?;

	OffsetDateTime::parse(ts.as_str(), &Rfc3339).map_err(|_| Error::InvalidRequest {
		message: "$.source_ref[\"ts\"] must be an RFC3339 datetime string.".to_string(),
	})?;

	let doc_type = if let Some(doc_type) = req.doc_type.as_ref() {
		let doc_type = DocType::parse(doc_type.as_str())?;

		if doc_type != source_ref_doc_type {
			return Err(Error::InvalidRequest {
				message: "doc_type must match source_ref.doc_type.".to_string(),
			});
		}

		doc_type
	} else {
		source_ref_doc_type
	};

	source_ref::validate_doc_source_ref_requirements(source_ref_doc_type.as_str(), source_ref)?;
	source_ref::validate_source_library_metadata(source_ref_doc_type.as_str(), source_ref)?;

	let write_policy =
		writegate::apply_write_policy(req.content.as_str(), req.write_policy.as_ref()).map_err(
			|err| Error::InvalidRequest { message: format!("write_policy is invalid: {err:?}") },
		)?;
	let write_policy_audit =
		if req.write_policy.is_some() { Some(write_policy.audit) } else { None };
	let content = write_policy.transformed;

	if content.trim().is_empty() {
		return Err(Error::InvalidRequest { message: "content must be non-empty.".to_string() });
	}
	if content.len() > DEFAULT_DOC_MAX_BYTES {
		return Err(Error::InvalidRequest {
			message: "content exceeds max_doc_bytes.".to_string(),
		});
	}
	if writegate::contains_secrets(content.as_str()) {
		return Err(Error::InvalidRequest { message: "content contains secrets.".to_string() });
	}

	if let Some(found) = non_english::find_non_english_path(&req.source_ref, "$.source_ref") {
		return Err(Error::NonEnglishInput { field: found });
	}

	if !english_gate::is_english_natural_language(content.as_str()) {
		return Err(Error::NonEnglishInput { field: "$.content".to_string() });
	}

	if let Some(title) = req.title.as_ref()
		&& !english_gate::is_english_natural_language(title.as_str())
	{
		return Err(Error::NonEnglishInput { field: "$.title".to_string() });
	}

	Ok(ValidatedDocsPut { doc_type, content, write_policy_audit })
}
