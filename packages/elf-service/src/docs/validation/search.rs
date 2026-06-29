use crate::docs::validation::{
	DOC_STATUSES, DocType, DocsSearchL0Filters, DocsSearchL0FiltersParsed,
	DocsSearchL0RangesParsed, DocsSearchL0Request, DocsSparseMode, Error, OffsetDateTime, Result,
	Rfc3339, english_gate,
};

pub(in crate::docs) fn validate_docs_search_l0(
	req: &DocsSearchL0Request,
) -> Result<DocsSearchL0Filters> {
	validate_docs_search_l0_query(req)?;

	let filters = parse_docs_search_l0_filters(req)?;
	let ranges = parse_docs_search_l0_ranges(req)?;

	validate_docs_search_l0_temporal_ranges(
		ranges.updated_after.as_ref(),
		ranges.updated_before.as_ref(),
		ranges.ts_gte.as_ref(),
		ranges.ts_lte.as_ref(),
	)?;

	Ok(DocsSearchL0Filters {
		scope: filters.scope,
		status: filters.status,
		doc_type: filters.doc_type,
		sparse_mode: filters.sparse_mode,
		domain: filters.domain,
		repo: filters.repo,
		agent_id: filters.agent_id,
		thread_id: filters.thread_id,
		updated_after: ranges.updated_after,
		updated_before: ranges.updated_before,
		ts_gte: ranges.ts_gte,
		ts_lte: ranges.ts_lte,
	})
}

pub(in crate::docs) fn validate_docs_search_l0_query(req: &DocsSearchL0Request) -> Result<()> {
	if req.query.trim().is_empty() {
		return Err(Error::InvalidRequest { message: "query must be non-empty.".to_string() });
	}
	if !english_gate::is_english_natural_language(req.query.as_str()) {
		return Err(Error::NonEnglishInput { field: "$.query".to_string() });
	}

	Ok(())
}

pub(in crate::docs) fn parse_docs_search_l0_filters(
	req: &DocsSearchL0Request,
) -> Result<DocsSearchL0FiltersParsed> {
	let scope = if let Some(scope) = req.scope.as_ref() {
		let scope = scope.trim();

		if scope.is_empty() {
			return Err(Error::InvalidRequest { message: "scope must be non-empty.".to_string() });
		}
		if !matches!(scope, "agent_private" | "project_shared" | "org_shared") {
			return Err(Error::InvalidRequest { message: "Unknown scope.".to_string() });
		}

		Some(scope.to_string())
	} else {
		None
	};
	let status = req
		.status
		.as_ref()
		.map(|status| status.trim().to_string())
		.filter(|status| !status.is_empty())
		.unwrap_or_else(|| "active".to_string())
		.to_lowercase();
	let status = if DOC_STATUSES.contains(&status.as_str()) {
		status
	} else {
		return Err(Error::InvalidRequest {
			message: "status must be one of: active|deleted.".to_string(),
		});
	};
	let sparse_mode = parse_sparse_mode(req.sparse_mode.as_ref())?;
	let doc_type = if let Some(doc_type) = req.doc_type.as_ref() {
		let doc_type = doc_type.trim();

		if doc_type.is_empty() {
			return Err(Error::InvalidRequest {
				message: "doc_type must be non-empty.".to_string(),
			});
		}

		Some(DocType::parse(doc_type)?)
	} else {
		None
	};
	let domain = req
		.domain
		.as_ref()
		.map(|domain| domain.trim().to_string())
		.filter(|domain| !domain.is_empty());
	let repo =
		req.repo.as_ref().map(|repo| repo.trim().to_string()).filter(|repo| !repo.is_empty());

	if domain.is_some() && doc_type != Some(DocType::Search) {
		return Err(Error::InvalidRequest {
			message: "domain requires doc_type=search.".to_string(),
		});
	}
	if repo.is_some() && doc_type != Some(DocType::Dev) {
		return Err(Error::InvalidRequest { message: "repo requires doc_type=dev.".to_string() });
	}

	let agent_id = req
		.agent_id
		.as_ref()
		.map(|agent_id| agent_id.trim().to_string())
		.filter(|agent_id| !agent_id.is_empty());
	let thread_id = req
		.thread_id
		.as_ref()
		.map(|thread_id| thread_id.trim().to_string())
		.filter(|thread_id| !thread_id.is_empty());

	if thread_id.is_some() && doc_type != Some(DocType::Chat) {
		return Err(Error::InvalidRequest {
			message: "thread_id requires doc_type=chat.".to_string(),
		});
	}

	Ok(DocsSearchL0FiltersParsed {
		scope,
		status,
		doc_type,
		sparse_mode,
		domain,
		repo,
		agent_id,
		thread_id,
	})
}

pub(in crate::docs) fn parse_docs_search_l0_ranges(
	req: &DocsSearchL0Request,
) -> Result<DocsSearchL0RangesParsed> {
	let updated_after = parse_optional_rfc3339(req.updated_after.as_ref(), "$.updated_after")?;
	let updated_before = parse_optional_rfc3339(req.updated_before.as_ref(), "$.updated_before")?;
	let ts_gte = parse_optional_rfc3339(req.ts_gte.as_ref(), "$.ts_gte")?;
	let ts_lte = parse_optional_rfc3339(req.ts_lte.as_ref(), "$.ts_lte")?;

	Ok(DocsSearchL0RangesParsed { updated_after, updated_before, ts_gte, ts_lte })
}

pub(in crate::docs) fn validate_docs_search_l0_temporal_ranges(
	updated_after: Option<&OffsetDateTime>,
	updated_before: Option<&OffsetDateTime>,
	ts_gte: Option<&OffsetDateTime>,
	ts_lte: Option<&OffsetDateTime>,
) -> Result<()> {
	if let (Some(updated_after), Some(updated_before)) = (updated_after, updated_before)
		&& updated_after >= updated_before
	{
		return Err(Error::InvalidRequest {
			message: "updated_after must be earlier than updated_before.".to_string(),
		});
	}
	if let (Some(ts_gte), Some(ts_lte)) = (ts_gte, ts_lte)
		&& ts_gte >= ts_lte
	{
		return Err(Error::InvalidRequest {
			message: "ts_gte must be earlier than ts_lte.".to_string(),
		});
	}

	Ok(())
}

pub(in crate::docs) fn parse_sparse_mode(raw: Option<&String>) -> Result<DocsSparseMode> {
	let raw = raw.as_ref().map(|mode| mode.trim().to_lowercase());
	let Some(mode) = raw else {
		return Ok(DocsSparseMode::Auto);
	};
	let mode = mode.as_str();

	match mode {
		"auto" => Ok(DocsSparseMode::Auto),
		"on" => Ok(DocsSparseMode::On),
		"off" => Ok(DocsSparseMode::Off),
		_ => Err(Error::InvalidRequest {
			message: "sparse_mode must be one of: auto|on|off.".to_string(),
		}),
	}
}

pub(in crate::docs) fn parse_optional_rfc3339(
	raw: Option<&String>,
	path: &str,
) -> Result<Option<OffsetDateTime>> {
	let Some(raw) = raw else {
		return Ok(None);
	};
	let raw = raw.trim();

	if raw.is_empty() {
		return Err(Error::InvalidRequest { message: format!("{path} must be non-empty.") });
	}

	OffsetDateTime::parse(raw, &Rfc3339).map(Some).map_err(|_| Error::InvalidRequest {
		message: format!("{path} must be an RFC3339 datetime string."),
	})
}
