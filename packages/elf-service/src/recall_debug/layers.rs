mod docs;
mod dreaming;
mod graph;
mod knowledge;
mod memory;

use crate::{
	access,
	recall_debug::{
		self, BTreeMap, BTreeSet, DEFAULT_RECALL_DEBUG_LIMIT, DocsSearchL0Request,
		DreamingReviewQueueRequest, ELF_RECALL_DEBUG_PANEL_SCHEMA_V1, ElfService, Error,
		GraphReportRequest, KnowledgePageSearchRequest, MAX_RECALL_DEBUG_DOCS_LIMIT,
		MAX_RECALL_DEBUG_LIMIT, MemoryNote, NoteDebugSourceRow, ORG_PROJECT_ID, OffsetDateTime,
		RecallDebugLayer, RecallDebugPanelRequest, RecallDebugPanelRequestEcho,
		RecallDebugPanelResponse, RecallDebugRow, Result, TraceBundleGetRequest, TraceBundleMode,
		Uuid, candidate_debug_row, candidate_is_selected, freshness_from_note_source,
		graph_replay_command, graph_temporal_status, json_anchor, knowledge_freshness,
		last_stage_name, layer_from_rows, layer_from_rows_with_artifacts,
		memory_compact_replay_artifact, not_requested_layer, note_debug_read_allowed,
		note_debug_source_pair, search_item_candidate_key, source_ref_from_note_source,
	},
	search,
};

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
			recall_debug::blocked_layer(
				"memory_notes",
				req.trace_id.map(|trace_id| trace_id.to_string()),
				"Requested memory trace bundle could not be read.",
				&err,
			)
		}));
		layers.push(
			self.recall_docs_layer(&req, docs_query.as_deref(), limit).await.unwrap_or_else(
				|err| {
					recall_debug::blocked_layer(
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
					recall_debug::blocked_layer(
						"knowledge_pages",
						knowledge_query.clone(),
						"Requested Knowledge Workspace page search could not be read.",
						&err,
					)
				}),
		);
		layers.push(self.recall_graph_layer(&req, limit).await.unwrap_or_else(|err| {
			recall_debug::blocked_layer(
				"graph_facts",
				req.graph_subject.as_ref().and_then(json_anchor),
				"Requested graph report could not be read.",
				&err,
			)
		}));
		layers.push(
			self.recall_dreaming_layer(&req, include_dreaming, limit).await.unwrap_or_else(|err| {
				recall_debug::blocked_layer(
					"dreaming_proposals",
					Some("include_dreaming=true".to_string()),
					"Requested Dreaming review queue could not be read.",
					&err,
				)
			}),
		);

		let summary = recall_debug::summarize_layers(&layers);
		let recall_trace = recall_debug::build_recall_trace(&layers);

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
