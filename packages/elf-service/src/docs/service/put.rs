use crate::docs::service::{
	self, DocDocument, DocsPutRequest, DocsPutResponse, ElfService, Error, ORG_PROJECT_ID,
	OffsetDateTime, Result, SourceCaptureSummaryInput, ValidatedDocsPut, access, doc_outbox, docs,
};

impl ElfService {
	/// Validates, chunks, stores, and enqueues a document for indexing.
	pub async fn docs_put(&self, req: DocsPutRequest) -> Result<DocsPutResponse> {
		let ValidatedDocsPut { doc_type, content, write_policy_audit } =
			service::validate_docs_put(&req)?;
		let now = OffsetDateTime::now_utc();
		let embed_version = crate::embedding_version(&self.cfg);
		let chunking_profile = service::resolve_doc_chunking_profile(doc_type);
		let tokenizer = service::load_tokenizer(&self.cfg)?;
		let tenant_id = req.tenant_id.clone();
		let project_id = req.project_id.clone();
		let agent_id = req.agent_id.clone();
		let scope = req.scope.clone();
		let title = req.title.clone();
		let source_ref = req.source_ref.clone();
		let source_ref_map = source_ref.as_object().ok_or_else(|| Error::InvalidRequest {
			message: "source_ref must be a JSON object.".to_string(),
		})?;
		let effective_project_id =
			if scope.trim() == "org_shared" { ORG_PROJECT_ID } else { project_id.as_str() };
		let content_bytes = content.len();
		let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
		let raw_content_hash = blake3::hash(req.content.as_bytes()).to_hex().to_string();
		let doc_id = service::source_record_id_for(
			tenant_id.as_str(),
			effective_project_id,
			agent_id.as_str(),
			scope.as_str(),
			doc_type,
			source_ref_map,
			content_hash.as_str(),
		);
		let mut chunks = service::split_tokens_by_offsets(
			content.as_str(),
			chunking_profile.max_tokens,
			chunking_profile.overlap_tokens,
			chunking_profile.max_chunks,
			&tokenizer,
		)?;

		for (chunk_index, chunk) in chunks.iter_mut().enumerate() {
			chunk.chunk_id = service::doc_chunk_id_for(doc_id, chunk_index as i32);
		}

		let chunk_rows = service::build_doc_chunk_rows(doc_id, &chunks, now);
		let source_capture = service::build_source_capture_summary(SourceCaptureSummaryInput {
			doc_id,
			source_ref: source_ref_map,
			doc_type,
			scope: scope.as_str(),
			title: title.as_deref(),
			content_hash: content_hash.as_str(),
			raw_content_hash: raw_content_hash.as_str(),
			now,
			chunks: &chunk_rows,
			write_policy_audit: write_policy_audit.as_ref(),
		})?;
		let normalized_source_ref =
			service::normalize_source_ref_for_capture(source_ref, &source_capture)?;
		let doc_row = DocDocument {
			doc_id,
			tenant_id: tenant_id.clone(),
			project_id: effective_project_id.to_string(),
			agent_id: agent_id.clone(),
			scope: scope.clone(),
			doc_type: doc_type.as_str().to_string(),
			status: "active".to_string(),
			title,
			source_ref: docs::normalize_source_ref(Some(normalized_source_ref)),
			content,
			content_bytes: content_bytes as i32,
			content_hash: content_hash.clone(),
			created_at: now,
			updated_at: now,
		};
		let mut tx = self.db.pool.begin().await?;

		docs::insert_doc_document(&mut *tx, &doc_row).await?;

		for chunk_row in &chunk_rows {
			docs::insert_doc_chunk(&mut *tx, chunk_row).await?;
			doc_outbox::enqueue_doc_outbox(
				&mut *tx,
				doc_id,
				chunk_row.chunk_id,
				"UPSERT",
				embed_version.as_str(),
			)
			.await?;
		}

		if scope.trim() != "agent_private" {
			access::ensure_active_project_scope_grant(
				&mut *tx,
				tenant_id.as_str(),
				effective_project_id,
				scope.as_str(),
				agent_id.as_str(),
			)
			.await?;
		}

		tx.commit().await?;

		Ok(DocsPutResponse {
			doc_id,
			source_capture,
			chunk_count: chunk_rows.len() as u32,
			content_bytes: content_bytes as u32,
			content_hash,
			write_policy_audit,
		})
	}
}
