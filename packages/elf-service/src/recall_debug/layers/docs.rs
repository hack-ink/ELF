use super::*;

impl ElfService {
	pub(super) async fn recall_docs_layer(
		&self,
		req: &RecallDebugPanelRequest,
		docs_query: Option<&str>,
		limit: u32,
	) -> Result<RecallDebugLayer> {
		let Some(query) = docs_query else {
			return Ok(not_requested_layer(
				"source_documents",
				"Supply query or docs_query to show Source Library document candidates.",
			));
		};
		let effective_limit = limit.min(MAX_RECALL_DEBUG_DOCS_LIMIT);
		let response = self
			.docs_search_l0(DocsSearchL0Request {
				tenant_id: req.tenant_id.clone(),
				project_id: req.project_id.clone(),
				caller_agent_id: req.agent_id.clone(),
				read_profile: req.read_profile.clone(),
				query: query.to_string(),
				scope: None,
				status: Some("active".to_string()),
				doc_type: None,
				sparse_mode: None,
				domain: None,
				repo: None,
				agent_id: None,
				thread_id: None,
				updated_after: None,
				updated_before: None,
				ts_gte: None,
				ts_lte: None,
				top_k: Some(effective_limit),
				candidate_k: Some(effective_limit.saturating_mul(3).max(effective_limit)),
				explain: Some(true),
			})
			.await?;
		let rows = response
			.items
			.into_iter()
			.enumerate()
			.map(|(index, item)| RecallDebugRow {
				layer: "source_documents".to_string(),
				item_ref: serde_json::json!({
					"trace_id": response.trace_id,
					"doc_id": item.doc_id,
					"chunk_id": item.chunk_id,
					"pointer": item.pointer,
				}),
				selection_state: "selected".to_string(),
				authority_layer: "source_library".to_string(),
				freshness_state: "active".to_string(),
				source_refs: serde_json::json!([{
					"schema": "source_ref/v1",
					"resolver": "elf_doc_ext/v1",
					"doc_id": item.doc_id,
					"chunk_id": item.chunk_id,
					"content_hash": item.content_hash,
					"chunk_hash": item.chunk_hash,
					"doc_updated_at": item.updated_at,
				}]),
				score: Some(item.score),
				rank: Some(index as u32 + 1),
				rationale: Some("docs_search_l0 selected chunk".to_string()),
				stage_reason: response
					.trajectory
					.as_ref()
					.and_then(|trajectory| trajectory.stages.last())
					.map(|stage| stage.stage_name.clone())
					.or(Some("docs_search_l0".to_string())),
				replay_command: Some(format!("elf_docs_search_l0 query={query:?} explain=true")),
				evidence_class: "pass".to_string(),
				debug_artifacts: serde_json::json!({
					"doc_type": item.doc_type,
					"scope": item.scope,
					"snippet": item.snippet,
					"trajectory": response.trajectory,
					"requested_limit": limit,
					"effective_limit": effective_limit,
				}),
			})
			.collect();
		let summary = if effective_limit < limit {
			format!(
				"Source Library search rows selected by docs_search_l0; effective docs cap is {effective_limit}."
			)
		} else {
			"Source Library search rows selected by docs_search_l0.".to_string()
		};

		Ok(layer_from_rows("source_documents", "pass", Some(query.to_string()), &summary, rows))
	}
}
