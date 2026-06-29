use super::{
	super::*,
	access::doc_read_allowed,
	text::{bounded_window, locate_quote},
};

pub(in crate::docs) async fn load_docs_excerpt_context(
	cfg: &Config,
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	read_profile: &str,
	doc_id: Uuid,
) -> Result<DocDocument> {
	let allowed_scopes = search::resolve_read_profile_scopes(cfg, read_profile)?;
	let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
	let shared_grants = access::load_shared_read_grants_with_org_shared(
		pool,
		tenant_id,
		project_id,
		agent_id,
		org_shared_allowed,
	)
	.await?;
	let doc = load_doc_document_for_read(pool, doc_id, tenant_id, project_id)
		.await?
		.ok_or_else(|| Error::NotFound { message: "Doc not found.".to_string() })?;

	if doc.status != "active"
		|| !doc_read_allowed(
			agent_id,
			&allowed_scopes,
			&shared_grants,
			doc.agent_id.as_str(),
			doc.scope.as_str(),
		) {
		return Err(Error::NotFound { message: "Doc not found.".to_string() });
	}

	Ok(doc)
}

pub(in crate::docs) async fn docs_excerpts_resolve_windowed_match(
	pool: &PgPool,
	doc: &DocDocument,
	req: &DocsExcerptsGetRequest,
	level_max: usize,
	trajectory: &mut DocTrajectoryBuilder,
	verified: &mut bool,
	verification_errors: &mut Vec<String>,
) -> Result<DocExcerptRange> {
	let DocExcerptMatch { selector_kind, match_start_offset, match_end_offset } =
		docs_excerpts_resolve_match(pool, doc, req, verified, verification_errors).await?;

	trajectory.push(
		"match_resolution",
		serde_json::json!({
			"selector_kind": selector_kind.as_str(),
			"match_start": match_start_offset,
			"match_end": match_end_offset,
		}),
	);

	let (start_offset, end_offset) =
		bounded_window(match_start_offset, match_end_offset, doc.content.as_str(), level_max);

	trajectory.push(
		"window_projection",
		serde_json::json!({
			"window_start": start_offset,
			"window_end": end_offset,
			"content_len": doc.content.len(),
		}),
	);

	Ok(DocExcerptRange {
		selector_kind,
		match_start_offset,
		match_end_offset,
		start_offset,
		end_offset,
	})
}

pub(in crate::docs) async fn docs_excerpts_resolve_match(
	pool: &PgPool,
	doc: &DocDocument,
	req: &DocsExcerptsGetRequest,
	verified: &mut bool,
	verification_errors: &mut Vec<String>,
) -> Result<DocExcerptMatch> {
	let (match_start_offset, match_end_offset, selector_kind) =
		resolve_excerpts_match_range(pool, doc, req, verified, verification_errors).await?;

	Ok(DocExcerptMatch { selector_kind, match_start_offset, match_end_offset })
}

pub(in crate::docs) async fn load_doc_document_for_read(
	executor: impl PgExecutor<'_>,
	doc_id: Uuid,
	tenant_id: &str,
	project_id: &str,
) -> Result<Option<DocDocument>> {
	let row: Option<DocDocument> = sqlx::query_as::<_, DocDocument>(
		"\
SELECT
	doc_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	doc_type,
	status,
	title,
	COALESCE(source_ref, '{}'::jsonb) AS source_ref,
	content,
	content_bytes,
	content_hash,
	created_at,
	updated_at
FROM doc_documents
WHERE doc_id = $1
  AND tenant_id = $2
  AND (
    project_id = $3
    OR (project_id = $4 AND scope = 'org_shared')
  )
LIMIT 1",
	)
	.bind(doc_id)
	.bind(tenant_id)
	.bind(project_id)
	.bind(ORG_PROJECT_ID)
	.fetch_optional(executor)
	.await?;

	Ok(row)
}

pub(in crate::docs) async fn resolve_excerpts_match_range(
	pool: &PgPool,
	doc: &DocDocument,
	req: &DocsExcerptsGetRequest,
	verified: &mut bool,
	verification_errors: &mut Vec<String>,
) -> Result<(usize, usize, ExcerptsSelectorKind)> {
	if let Some(chunk_id) = req.chunk_id {
		let chunk = docs::get_doc_chunk(pool, chunk_id).await?;
		let Some(chunk) = chunk else {
			return Err(Error::NotFound { message: "Chunk not found.".to_string() });
		};

		if chunk.doc_id != doc.doc_id {
			return Err(Error::NotFound { message: "Chunk not found.".to_string() });
		}

		return Ok((
			chunk.start_offset.max(0) as usize,
			chunk.end_offset.max(0) as usize,
			ExcerptsSelectorKind::ChunkId,
		));
	}
	if let Some(quote) = req.quote.as_ref() {
		return Ok(match locate_quote(&doc.content, quote) {
			Some((s, e)) => (s, e, ExcerptsSelectorKind::Quote),
			None => {
				*verified = false;

				verification_errors.push("QUOTE_SELECTOR_NOT_FOUND".to_string());

				if let Some(pos) = req.position.as_ref() {
					(
						pos.start.min(doc.content.len()),
						pos.end.min(doc.content.len()),
						ExcerptsSelectorKind::Position,
					)
				} else {
					return Err(Error::NotFound {
						message: "Selector did not match document.".to_string(),
					});
				}
			},
		});
	}
	if let Some(pos) = req.position.as_ref() {
		return Ok((
			pos.start.min(doc.content.len()),
			pos.end.min(doc.content.len()),
			ExcerptsSelectorKind::Position,
		));
	}

	Err(Error::InvalidRequest {
		message: "One of chunk_id, quote, or position is required.".to_string(),
	})
}
