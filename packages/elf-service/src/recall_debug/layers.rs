use super::*;

mod docs;
mod dreaming;
mod graph;
mod knowledge;
mod memory;

impl ElfService {
	/// Builds a cross-layer recall/debug panel from existing readback surfaces.
	pub async fn recall_debug_panel(
		&self,
		req: RecallDebugPanelRequest,
	) -> Result<RecallDebugPanelResponse> {
		let limit =
			req.limit.unwrap_or(DEFAULT_RECALL_DEBUG_LIMIT).clamp(1, MAX_RECALL_DEBUG_LIMIT);
		let docs_query = req
			.docs_query
			.clone()
			.or_else(|| req.query.clone())
			.map(|value| value.trim().to_string())
			.filter(|value| !value.is_empty());
		let knowledge_query = req
			.knowledge_query
			.clone()
			.or_else(|| req.query.clone())
			.map(|value| value.trim().to_string())
			.filter(|value| !value.is_empty());
		let include_dreaming = req.include_dreaming == Some(true);
		let mut layers = Vec::new();

		layers.push(self.recall_memory_layer(&req, limit).await.unwrap_or_else(|err| {
			blocked_layer(
				"memory_notes",
				req.trace_id.map(|trace_id| trace_id.to_string()),
				"Requested memory trace bundle could not be read.",
				&err,
			)
		}));
		layers.push(
			self.recall_docs_layer(&req, docs_query.as_deref(), limit).await.unwrap_or_else(
				|err| {
					blocked_layer(
						"source_documents",
						docs_query.clone(),
						"Requested Source Library document search could not be read.",
						&err,
					)
				},
			),
		);
		layers.push(
			self.recall_knowledge_layer(&req, knowledge_query.as_deref(), limit)
				.await
				.unwrap_or_else(|err| {
					blocked_layer(
						"knowledge_pages",
						knowledge_query.clone(),
						"Requested Knowledge Workspace page search could not be read.",
						&err,
					)
				}),
		);
		layers.push(self.recall_graph_layer(&req, limit).await.unwrap_or_else(|err| {
			blocked_layer(
				"graph_facts",
				req.graph_subject.as_ref().and_then(json_anchor),
				"Requested graph report could not be read.",
				&err,
			)
		}));
		layers.push(
			self.recall_dreaming_layer(&req, include_dreaming, limit).await.unwrap_or_else(|err| {
				blocked_layer(
					"dreaming_proposals",
					Some("include_dreaming=true".to_string()),
					"Requested Dreaming review queue could not be read.",
					&err,
				)
			}),
		);

		let summary = summarize_layers(&layers);
		let recall_trace = build_recall_trace(&layers);

		Ok(RecallDebugPanelResponse {
			schema: ELF_RECALL_DEBUG_PANEL_SCHEMA_V1.to_string(),
			generated_at: OffsetDateTime::now_utc(),
			request: RecallDebugPanelRequestEcho {
				trace_id: req.trace_id,
				docs_query,
				knowledge_query,
				graph_subject_supplied: req.graph_subject.is_some(),
				include_dreaming,
				limit,
			},
			summary,
			recall_trace,
			layers,
		})
	}
}
