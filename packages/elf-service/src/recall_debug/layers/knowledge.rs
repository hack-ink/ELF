use super::*;

impl ElfService {
	pub(super) async fn recall_knowledge_layer(
		&self,
		req: &RecallDebugPanelRequest,
		knowledge_query: Option<&str>,
		limit: u32,
	) -> Result<RecallDebugLayer> {
		let Some(query) = knowledge_query else {
			return Ok(not_requested_layer(
				"knowledge_pages",
				"Supply query or knowledge_query to show Knowledge Workspace page candidates.",
			));
		};
		let response = self
			.knowledge_pages_search(KnowledgePageSearchRequest {
				tenant_id: req.tenant_id.clone(),
				project_id: req.project_id.clone(),
				agent_id: req.agent_id.clone(),
				read_profile: req.read_profile.clone(),
				query: query.to_string(),
				page_kind: None,
				limit: Some(limit),
			})
			.await?;
		let rows = response
			.items
			.into_iter()
			.enumerate()
			.map(|(index, item)| RecallDebugRow {
				layer: "knowledge_pages".to_string(),
				item_ref: serde_json::json!({
					"page_id": item.page_id,
					"section_id": item.section_id,
					"page_kind": item.page_kind,
					"page_key": item.page_key,
				}),
				selection_state: "selected".to_string(),
				authority_layer: "derived_knowledge_page".to_string(),
				freshness_state: knowledge_freshness(&item),
				source_refs: serde_json::json!({
					"source_coverage": item.source_coverage,
					"section_source_ref_count": item.source_ref_count,
					"citation_count": item.citation_count,
					"source_refs": item.source_refs,
				}),
				score: None,
				rank: Some(index as u32 + 1),
				rationale: Some("knowledge_pages_search selected section".to_string()),
				stage_reason: Some("knowledge_page_search".to_string()),
				replay_command: Some(format!(
					"elf_recall_debug_panel knowledge_query={query:?} layer=knowledge_pages"
				)),
				evidence_class: "pass".to_string(),
				debug_artifacts: serde_json::json!({
					"title": item.title,
					"heading": item.heading,
					"lint_summary": item.lint_summary,
					"trust_state": item.trust_state,
					"repair_guidance": item.repair_guidance,
					"snippet": item.snippet,
				}),
			})
			.collect();

		Ok(layer_from_rows_with_artifacts(
			"knowledge_pages",
			"pass",
			Some(query.to_string()),
			"Knowledge Workspace sections selected by page search.",
			rows,
			serde_json::json!({}),
		))
	}
}
