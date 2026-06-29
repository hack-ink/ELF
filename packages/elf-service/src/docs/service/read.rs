use crate::docs::service::{
	self, DocDocument, DocsDeleteRequest, DocsDeleteResponse, DocsGetRequest, DocsGetResponse,
	ElfService, Error, HashSet, NoteOp, ORG_PROJECT_ID, OffsetDateTime, Result, access, doc_outbox,
	docs, search,
};

impl ElfService {
	/// Loads document metadata when the caller can read the requested scope.
	pub async fn docs_get(&self, req: DocsGetRequest) -> Result<DocsGetResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();
		let read_profile = req.read_profile.trim();

		if tenant_id.is_empty()
			|| project_id.is_empty()
			|| agent_id.is_empty()
			|| read_profile.is_empty()
		{
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, agent_id, and read_profile are required."
					.to_string(),
			});
		}

		let allowed_scopes = search::resolve_read_profile_scopes(&self.cfg, read_profile)?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
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
		.bind(req.doc_id)
		.bind(tenant_id)
		.bind(project_id)
		.bind(ORG_PROJECT_ID)
		.fetch_optional(&self.db.pool)
		.await?;
		let Some(row) = row else {
			return Err(Error::NotFound { message: "Doc not found.".to_string() });
		};
		let shared_grants = if row.scope == "agent_private" {
			HashSet::new()
		} else {
			access::load_shared_read_grants_with_org_shared(
				&self.db.pool,
				tenant_id,
				project_id,
				agent_id,
				org_shared_allowed,
			)
			.await?
		};

		if row.status != "active"
			|| !service::doc_read_allowed(
				agent_id,
				&allowed_scopes,
				&shared_grants,
				row.agent_id.as_str(),
				row.scope.as_str(),
			) {
			return Err(Error::NotFound { message: "Doc not found.".to_string() });
		}

		Ok(DocsGetResponse {
			doc_id: row.doc_id,
			tenant_id: row.tenant_id,
			project_id: row.project_id,
			agent_id: row.agent_id,
			scope: row.scope,
			doc_type: row.doc_type,
			status: row.status,
			title: row.title,
			source_ref: row.source_ref,
			content_bytes: row.content_bytes.max(0) as u32,
			content_hash: row.content_hash,
			created_at: row.created_at,
			updated_at: row.updated_at,
		})
	}

	/// Soft-deletes one Source Library document and enqueues doc-vector deletion.
	pub async fn docs_delete(&self, req: DocsDeleteRequest) -> Result<DocsDeleteResponse> {
		let now = OffsetDateTime::now_utc();
		let embed_version = crate::embedding_version(&self.cfg);
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let mut tx = self.db.pool.begin().await?;
		let row: DocDocument = sqlx::query_as::<_, DocDocument>(
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
FOR UPDATE",
		)
		.bind(req.doc_id)
		.bind(tenant_id)
		.bind(project_id)
		.bind(ORG_PROJECT_ID)
		.fetch_optional(&mut *tx)
		.await?
		.ok_or_else(|| Error::NotFound { message: "Doc not found.".to_string() })?;

		if row.agent_id != agent_id {
			return Err(Error::NotFound { message: "Doc not found.".to_string() });
		}

		let scope_allowed = self.cfg.scopes.allowed.iter().any(|scope| scope == &row.scope);
		let write_allowed = match row.scope.as_str() {
			"agent_private" => self.cfg.scopes.write_allowed.agent_private,
			"project_shared" => self.cfg.scopes.write_allowed.project_shared,
			"org_shared" => self.cfg.scopes.write_allowed.org_shared,
			_ => false,
		};

		if !scope_allowed || !write_allowed {
			return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
		}
		if row.status == "deleted" {
			tx.commit().await?;

			return Ok(DocsDeleteResponse {
				doc_id: row.doc_id,
				op: NoteOp::None,
				chunk_delete_count: 0,
			});
		}

		let chunks = docs::list_doc_chunks(&mut *tx, row.doc_id).await?;

		docs::mark_doc_deleted(&mut *tx, tenant_id, row.doc_id, now).await?;

		for chunk in &chunks {
			doc_outbox::enqueue_doc_outbox(
				&mut *tx,
				row.doc_id,
				chunk.chunk_id,
				"DELETE",
				embed_version.as_str(),
			)
			.await?;
		}

		tx.commit().await?;

		Ok(DocsDeleteResponse {
			doc_id: row.doc_id,
			op: NoteOp::Delete,
			chunk_delete_count: chunks.len() as u32,
		})
	}
}
