//! Cross-layer recall/debug panel readback.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	DocsSearchL0Request, DreamingReviewQueueRequest, ElfService, Error, GraphQueryEntityRef,
	GraphQueryPredicateRef, GraphReportRequest, KnowledgePageSearchItem,
	KnowledgePageSearchRequest, Result, SearchExplainItem, SearchTrajectoryStage,
	TraceBundleGetRequest,
	access::{self, ORG_PROJECT_ID, SharedSpaceGrantKey},
	search::{self, TraceBundleMode, TraceReplayCandidate},
};
use elf_storage::models::MemoryNote;

/// Schema identifier for recall/debug panel responses.
pub const ELF_RECALL_DEBUG_PANEL_SCHEMA_V1: &str = "elf.recall_debug_panel/v1";
/// Schema identifier for deterministic recall trace projections.
pub const ELF_RECALL_TRACE_SCHEMA_V1: &str = "elf.recall_trace/v1";

const DEFAULT_RECALL_DEBUG_LIMIT: u32 = 25;
const MAX_RECALL_DEBUG_LIMIT: u32 = 100;
const MAX_RECALL_DEBUG_DOCS_LIMIT: u32 = 32;

/// Request payload for the cross-layer recall/debug panel.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RecallDebugPanelRequest {
	/// Tenant that owns the readback.
	pub tenant_id: String,
	/// Project that owns the readback.
	pub project_id: String,
	/// Agent requesting the readback.
	pub agent_id: String,
	/// Read profile used for memory, document, and graph visibility.
	pub read_profile: String,
	/// Optional search trace anchor for memory selected/dropped rows.
	pub trace_id: Option<Uuid>,
	/// Shared query used when docs_query or knowledge_query are omitted.
	pub query: Option<String>,
	/// Optional Source Library query.
	pub docs_query: Option<String>,
	/// Optional Knowledge Workspace page query.
	pub knowledge_query: Option<String>,
	/// Optional graph subject selector.
	pub graph_subject: Option<GraphQueryEntityRef>,
	/// Optional graph predicate selector.
	pub graph_predicate: Option<GraphQueryPredicateRef>,
	/// Whether to include Dreaming review queue proposals. Omitted means not requested.
	pub include_dreaming: Option<bool>,
	/// Maximum rows per layer.
	pub limit: Option<u32>,
	#[serde(skip)]
	/// Whether project-scoped trace anchors are allowed for an admin mirror request.
	pub allow_project_trace_debug: bool,
}

/// Cross-layer recall/debug panel response.
#[derive(Clone, Debug, Serialize)]
pub struct RecallDebugPanelResponse {
	/// Response schema identifier.
	pub schema: String,
	#[serde(with = "crate::time_serde")]
	/// Panel generation timestamp.
	pub generated_at: OffsetDateTime,
	/// Echo of the effective anchors used for this response.
	pub request: RecallDebugPanelRequestEcho,
	/// Aggregate panel summary.
	pub summary: RecallDebugPanelSummary,
	/// Deterministic flat trace projection for agents and fixture assertions.
	pub recall_trace: RecallTrace,
	/// Cross-layer rows grouped by source layer.
	pub layers: Vec<RecallDebugLayer>,
}

/// Deterministic flat recall trace over all requested layers.
#[derive(Clone, Debug, Serialize)]
pub struct RecallTrace {
	/// Trace schema identifier.
	pub schema: String,
	/// Aggregate trace counters.
	pub summary: RecallTraceSummary,
	/// Stable trace entries in layer and row order.
	pub entries: Vec<RecallTraceEntry>,
}

/// Aggregate counters for a recall trace.
#[derive(Clone, Debug, Default, Serialize)]
pub struct RecallTraceSummary {
	/// Number of trace entries.
	pub entry_count: usize,
	/// Entries whose row selection state is selected.
	pub selected_count: usize,
	/// Entries whose row selection state is dropped.
	pub dropped_count: usize,
	/// Entries whose freshness state indicates stale or non-current evidence.
	pub stale_count: usize,
	/// Entries representing blocked layers.
	pub blocked_count: usize,
	/// Entries representing layers that were not requested.
	pub not_requested_count: usize,
	/// Entries that require raw SQL for diagnosis.
	pub raw_sql_needed_count: usize,
	/// Entries with a replay command or deterministic artifact path.
	pub replay_command_count: usize,
}

/// One compact recall trace entry.
#[derive(Clone, Debug, Serialize)]
pub struct RecallTraceEntry {
	/// Layer identifier.
	pub layer: String,
	/// Primary trace state for compact assertions.
	pub context_state: String,
	/// Original row selection state or layer evidence class.
	pub selection_state: String,
	/// Authority layer that owns the context.
	pub authority_layer: String,
	/// Freshness or temporal state.
	pub freshness_state: String,
	/// Stable identifiers for replay or hydration.
	pub item_ref: Value,
	/// Source refs or source snapshots supporting the context.
	pub source_refs: Value,
	/// Optional score.
	pub score: Option<f32>,
	/// Optional rank.
	pub rank: Option<u32>,
	/// Compact policy or stage reason for the state.
	pub policy_reason: Option<String>,
	/// Replay command or deterministic artifact path.
	pub replay_command: Option<String>,
	/// Layer or row evidence class.
	pub evidence_class: String,
	/// Whether raw SQL is required to diagnose this entry.
	pub raw_sql_needed: bool,
}

/// Stable request echo for panel responses.
#[derive(Clone, Debug, Serialize)]
pub struct RecallDebugPanelRequestEcho {
	/// Search trace anchor used for memory rows.
	pub trace_id: Option<Uuid>,
	/// Effective Source Library query.
	pub docs_query: Option<String>,
	/// Effective Knowledge Workspace query.
	pub knowledge_query: Option<String>,
	/// Whether a graph subject was supplied.
	pub graph_subject_supplied: bool,
	/// Whether Dreaming proposals were included.
	pub include_dreaming: bool,
	/// Effective row cap per layer.
	pub limit: u32,
}

/// Aggregate panel counters.
#[derive(Clone, Debug, Default, Serialize)]
pub struct RecallDebugPanelSummary {
	/// Number of returned layers.
	pub layer_count: usize,
	/// Total returned row count.
	pub row_count: usize,
	/// Rows selected by a retrieval or review stage.
	pub selected_count: usize,
	/// Rows dropped by a retrieval or review stage.
	pub dropped_count: usize,
	/// Rows available for inspection but not selected/dropped.
	pub available_count: usize,
	/// Layers skipped because no anchor was supplied.
	pub not_requested_layer_count: usize,
	/// Layers that require follow-up before they can prove a debug claim.
	pub incomplete_layer_count: usize,
	/// Rows or layers that require raw SQL to inspect.
	pub raw_sql_needed_count: usize,
	/// Rows with a replay command or deterministic artifact path.
	pub replay_command_count: usize,
	/// Evidence-class counts across layers.
	pub evidence_class_counts: BTreeMap<String, usize>,
}

/// One recall/debug source layer.
#[derive(Clone, Debug, Serialize)]
pub struct RecallDebugLayer {
	/// Layer identifier.
	pub layer: String,
	/// Evidence class for this layer.
	pub evidence_class: String,
	/// Human-readable layer summary.
	pub summary: String,
	/// Query or object anchor used by the layer.
	pub anchor: Option<String>,
	/// Number of returned rows.
	pub row_count: usize,
	/// Selected rows in this layer.
	pub selected_count: usize,
	/// Dropped rows in this layer.
	pub dropped_count: usize,
	/// Available review/inspection rows in this layer.
	pub available_count: usize,
	/// Whether raw SQL is needed to inspect this layer.
	pub raw_sql_needed: bool,
	/// Whether the layer includes replay commands or deterministic artifact paths.
	pub replayable: bool,
	/// Returned layer rows.
	pub rows: Vec<RecallDebugRow>,
}

/// One item in the recall/debug panel.
#[derive(Clone, Debug, Serialize)]
pub struct RecallDebugRow {
	/// Layer identifier.
	pub layer: String,
	/// Stable item reference.
	pub item_ref: Value,
	/// Selection state such as selected, dropped, available, or reviewable.
	pub selection_state: String,
	/// Authority layer that owns the row.
	pub authority_layer: String,
	/// Freshness or temporal state.
	pub freshness_state: String,
	/// Source refs or source snapshots backing the row.
	pub source_refs: Value,
	/// Optional final score.
	pub score: Option<f32>,
	/// Optional rank within the layer.
	pub rank: Option<u32>,
	/// Short selection rationale.
	pub rationale: Option<String>,
	/// Stage reason for selected/dropped status.
	pub stage_reason: Option<String>,
	/// Replay command or deterministic artifact path when available.
	pub replay_command: Option<String>,
	/// Row-level evidence class.
	pub evidence_class: String,
	/// Layer-specific debug artifacts.
	pub debug_artifacts: Value,
}

#[derive(Clone, Debug)]
struct NoteDebugSourceRow {
	status: String,
	source_ref: Value,
	updated_at: OffsetDateTime,
}

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

	async fn recall_memory_layer(
		&self,
		req: &RecallDebugPanelRequest,
		limit: u32,
	) -> Result<RecallDebugLayer> {
		let Some(trace_id) = req.trace_id else {
			return Ok(not_requested_layer(
				"memory_notes",
				"Supply trace_id to show selected and dropped Memory Note candidates.",
			));
		};

		if !req.allow_project_trace_debug {
			self.ensure_public_recall_trace_allowed(req, trace_id).await?;
		}

		let bundle = self
			.trace_bundle_get(TraceBundleGetRequest {
				tenant_id: req.tenant_id.clone(),
				project_id: req.project_id.clone(),
				agent_id: req.agent_id.clone(),
				trace_id,
				mode: TraceBundleMode::Bounded,
				stage_items_limit: Some(limit),
				candidates_limit: Some(limit.saturating_mul(4).min(400)),
			})
			.await?;
		let selected_note_ids =
			bundle.items.iter().map(|item| item.note_id).collect::<BTreeSet<_>>();
		let selected_candidate_keys =
			bundle.items.iter().filter_map(search_item_candidate_key).collect::<BTreeSet<_>>();
		let candidate_note_ids =
			bundle.candidates.as_ref().into_iter().flatten().map(|candidate| candidate.note_id);
		let all_note_ids =
			selected_note_ids.iter().copied().chain(candidate_note_ids).collect::<BTreeSet<_>>();
		let source_refs = self
			.load_memory_note_debug_sources(req, all_note_ids.iter().copied().collect())
			.await?;
		let replay_command = format!("elf_admin_trace_bundle_get trace_id={trace_id} mode=bounded");
		let visible_items = bundle
			.items
			.iter()
			.filter(|item| source_refs.contains_key(&item.note_id))
			.collect::<Vec<_>>();
		let dropped_candidates = bundle
			.candidates
			.as_deref()
			.unwrap_or_default()
			.iter()
			.filter(|candidate| !candidate_is_selected(&selected_candidate_keys, candidate))
			.filter(|candidate| source_refs.contains_key(&candidate.note_id))
			.collect::<Vec<_>>();
		let selected_cap = if !dropped_candidates.is_empty() && limit > 1 {
			limit as usize - 1
		} else {
			limit as usize
		};
		let mut rows = Vec::new();

		for item in visible_items.iter().take(selected_cap) {
			let source = source_refs.get(&item.note_id);

			rows.push(RecallDebugRow {
				layer: "memory_notes".to_string(),
				item_ref: serde_json::json!({
					"trace_id": trace_id,
					"result_handle": item.result_handle,
					"note_id": item.note_id,
					"chunk_id": item.chunk_id,
				}),
				selection_state: "selected".to_string(),
				authority_layer: "memory_note".to_string(),
				freshness_state: freshness_from_note_source(source),
				source_refs: source_ref_from_note_source(source),
				score: Some(item.explain.ranking.final_score),
				rank: Some(item.rank),
				rationale: Some("final ranked search result".to_string()),
				stage_reason: last_stage_name(bundle.stages.as_slice())
					.or_else(|| Some("final_ranking".to_string())),
				replay_command: Some(replay_command.clone()),
				evidence_class: "pass".to_string(),
				debug_artifacts: serde_json::json!({
					"ranking_explain": item.explain,
					"note_updated_at": source.map(|row| row.updated_at),
				}),
			});
		}

		let dropped_cap = limit.saturating_sub(rows.len() as u32) as usize;

		for candidate in dropped_candidates.into_iter().take(dropped_cap) {
			rows.push(candidate_debug_row(
				trace_id,
				candidate,
				source_refs.get(&candidate.note_id),
				replay_command.as_str(),
			));
		}

		Ok(layer_from_rows(
			"memory_notes",
			"pass",
			Some(trace_id.to_string()),
			"Search trace bundle with selected results and replay candidates.",
			rows,
		))
	}

	async fn ensure_public_recall_trace_allowed(
		&self,
		req: &RecallDebugPanelRequest,
		trace_id: Uuid,
	) -> Result<()> {
		let row: Option<(i64,)> = sqlx::query_as(
			"\
SELECT 1
FROM search_traces
WHERE trace_id = $1
  AND tenant_id = $2
  AND project_id = $3
  AND agent_id = $4
  AND read_profile = $5",
		)
		.bind(trace_id)
		.bind(req.tenant_id.trim())
		.bind(req.project_id.trim())
		.bind(req.agent_id.trim())
		.bind(req.read_profile.trim())
		.fetch_optional(&self.db.pool)
		.await?;

		if row.is_some() {
			Ok(())
		} else {
			Err(Error::InvalidRequest {
				message: "Unknown trace_id for this recall context.".to_string(),
			})
		}
	}

	async fn recall_docs_layer(
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

	async fn recall_knowledge_layer(
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

		Ok(layer_from_rows(
			"knowledge_pages",
			"pass",
			Some(query.to_string()),
			"Knowledge Workspace sections selected by page search.",
			rows,
		))
	}

	async fn recall_graph_layer(
		&self,
		req: &RecallDebugPanelRequest,
		limit: u32,
	) -> Result<RecallDebugLayer> {
		let Some(subject) = req.graph_subject.clone() else {
			return Ok(not_requested_layer(
				"graph_facts",
				"Supply graph_subject to show graph fact candidates and temporal status.",
			));
		};
		let response = self
			.graph_report(GraphReportRequest {
				tenant_id: req.tenant_id.clone(),
				project_id: req.project_id.clone(),
				agent_id: req.agent_id.clone(),
				read_profile: req.read_profile.clone(),
				subject,
				predicate: req.graph_predicate.clone(),
				scopes: None,
				as_of: None,
				limit: Some(limit),
				explain: Some(true),
			})
			.await?;
		let subject_anchor = response.subject.canonical.clone();
		let replay_command = graph_replay_command(&subject_anchor, req.graph_predicate.as_ref());
		let rows = response
			.facts
			.into_iter()
			.enumerate()
			.map(|(index, fact)| RecallDebugRow {
				layer: "graph_facts".to_string(),
				item_ref: serde_json::json!({
					"fact_id": fact.fact_id,
					"subject": subject_anchor,
					"predicate": fact.predicate,
					"object": fact.object,
				}),
				selection_state: "available".to_string(),
				authority_layer: "graph_fact".to_string(),
				freshness_state: graph_temporal_status(fact.temporal_status),
				source_refs: serde_json::json!({
					"evidence_note_ids": fact.evidence_note_ids,
					"supersedes_fact_ids": fact.supersedes_fact_ids,
					"superseded_by_fact_ids": fact.superseded_by_fact_ids,
				}),
				score: None,
				rank: Some(index as u32 + 1),
				rationale: Some("graph_report returned source-backed fact".to_string()),
				stage_reason: Some(fact.status_markers.join(",")),
				replay_command: Some(replay_command.clone()),
				evidence_class: "pass".to_string(),
				debug_artifacts: serde_json::json!({
					"scope": fact.scope,
					"actor": fact.actor,
					"valid_from": fact.valid_from,
					"valid_to": fact.valid_to,
					"status_markers": fact.status_markers,
				}),
			})
			.collect();

		Ok(layer_from_rows(
			"graph_facts",
			"pass",
			Some(subject_anchor),
			"Graph facts from source-backed graph report.",
			rows,
		))
	}

	async fn recall_dreaming_layer(
		&self,
		req: &RecallDebugPanelRequest,
		include_dreaming: bool,
		limit: u32,
	) -> Result<RecallDebugLayer> {
		if !include_dreaming {
			return Ok(not_requested_layer(
				"dreaming_proposals",
				"Set include_dreaming=true to show reviewable Dreaming proposals.",
			));
		}

		let response = self
			.dreaming_review_queue(DreamingReviewQueueRequest {
				tenant_id: req.tenant_id.clone(),
				project_id: req.project_id.clone(),
				run_id: None,
				review_state: None,
				limit: Some(limit),
			})
			.await?;
		let rows = response
			.items
			.into_iter()
			.enumerate()
			.map(|(index, item)| RecallDebugRow {
				layer: "dreaming_proposals".to_string(),
				item_ref: serde_json::json!({
					"proposal_id": item.proposal_id,
					"run_id": item.run_id,
					"queue_variant": item.queue_variant,
					"target_ref": item.target_ref,
				}),
				selection_state: "reviewable".to_string(),
				authority_layer: "reviewable_dreaming_proposal".to_string(),
				freshness_state: item.review_state.clone(),
				source_refs: serde_json::json!({
					"source_refs": item.source_refs,
					"source_snapshot": item.source_snapshot,
					"affected_refs": item.affected_refs,
				}),
				score: Some(item.confidence),
				rank: Some(index as u32 + 1),
				rationale: Some(item.policy.reason.clone()),
				stage_reason: Some(format!(
					"review_state={}, auto_apply_allowed={}",
					item.review_state, item.policy.auto_apply_allowed
				)),
				replay_command: Some("elf_dreaming_review_queue limit=<n>".to_string()),
				evidence_class: "pass".to_string(),
				debug_artifacts: serde_json::json!({
					"policy": item.policy,
					"unsupported_claim_flags": item.unsupported_claim_flags,
					"contradiction_markers": item.contradiction_markers,
					"staleness_markers": item.staleness_markers,
					"diff": item.diff,
					"review_audit": item.review_audit,
				}),
			})
			.collect();

		Ok(layer_from_rows(
			"dreaming_proposals",
			"pass",
			None,
			"Dreaming review queue proposals available for reviewer action.",
			rows,
		))
	}

	async fn load_memory_note_debug_sources(
		&self,
		req: &RecallDebugPanelRequest,
		note_ids: Vec<Uuid>,
	) -> Result<BTreeMap<Uuid, NoteDebugSourceRow>> {
		if note_ids.is_empty() {
			return Ok(BTreeMap::new());
		}

		let rows = sqlx::query_as::<_, MemoryNote>(
			"\
SELECT *
FROM memory_notes
	WHERE tenant_id = $1
	  AND note_id = ANY($3::uuid[])
	  AND (
	    project_id = $2
	    OR (project_id = $4 AND scope = 'org_shared')
	  )",
		)
		.bind(req.tenant_id.as_str())
		.bind(req.project_id.as_str())
		.bind(note_ids)
		.bind(ORG_PROJECT_ID)
		.fetch_all(&self.db.pool)
		.await?;

		if req.allow_project_trace_debug {
			return Ok(rows.into_iter().map(note_debug_source_pair).collect());
		}

		let allowed_scopes =
			search::resolve_read_profile_scopes(&self.cfg, req.read_profile.trim())?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			&self.db.pool,
			req.tenant_id.trim(),
			req.project_id.trim(),
			req.agent_id.trim(),
			org_shared_allowed,
		)
		.await?;

		Ok(rows
			.into_iter()
			.filter(|note| {
				note_debug_read_allowed(note, req.agent_id.trim(), &allowed_scopes, &shared_grants)
			})
			.map(note_debug_source_pair)
			.collect())
	}
}

fn note_debug_source_pair(note: MemoryNote) -> (Uuid, NoteDebugSourceRow) {
	(
		note.note_id,
		NoteDebugSourceRow {
			status: note.status,
			source_ref: note.source_ref,
			updated_at: note.updated_at,
		},
	)
}

fn note_debug_read_allowed(
	note: &MemoryNote,
	requester_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<SharedSpaceGrantKey>,
) -> bool {
	if !allowed_scopes.iter().any(|scope| scope == &note.scope) {
		return false;
	}
	if note.scope == "agent_private" {
		return note.agent_id == requester_agent_id;
	}
	if !matches!(note.scope.as_str(), "project_shared" | "org_shared") {
		return false;
	}
	if note.agent_id == requester_agent_id {
		return true;
	}

	shared_grants.contains(&SharedSpaceGrantKey {
		scope: note.scope.clone(),
		space_owner_agent_id: note.agent_id.clone(),
	})
}

fn candidate_debug_row(
	trace_id: Uuid,
	candidate: &TraceReplayCandidate,
	source: Option<&NoteDebugSourceRow>,
	replay_command: &str,
) -> RecallDebugRow {
	let selected_by_diversity = candidate.diversity_selected.unwrap_or(false);
	let skipped_reason = candidate.diversity_skipped_reason.clone().or_else(|| {
		if selected_by_diversity {
			candidate.diversity_selected_reason.clone()
		} else {
			Some("not_in_final_top_k".to_string())
		}
	});

	RecallDebugRow {
		layer: "memory_notes".to_string(),
		item_ref: serde_json::json!({
			"trace_id": trace_id,
			"note_id": candidate.note_id,
			"chunk_id": candidate.chunk_id,
			"chunk_index": candidate.chunk_index,
		}),
		selection_state: "dropped".to_string(),
		authority_layer: "memory_note".to_string(),
		freshness_state: freshness_from_note_source(source),
		source_refs: source_ref_from_note_source(source),
		score: candidate.retrieval_score,
		rank: Some(candidate.retrieval_rank),
		rationale: Some(
			"candidate captured for replay but not selected in final result set".to_string(),
		),
		stage_reason: skipped_reason,
		replay_command: Some(replay_command.to_string()),
		evidence_class: "pass".to_string(),
		debug_artifacts: serde_json::json!({
			"snippet": candidate.snippet,
			"rerank_score": candidate.rerank_score,
			"note_scope": candidate.note_scope,
			"diversity_selected": candidate.diversity_selected,
			"diversity_selected_rank": candidate.diversity_selected_rank,
			"diversity_nearest_selected_note_id": candidate.diversity_nearest_selected_note_id,
			"diversity_similarity": candidate.diversity_similarity,
			"diversity_mmr_score": candidate.diversity_mmr_score,
			"diversity_missing_embedding": candidate.diversity_missing_embedding,
		}),
	}
}

fn summarize_layers(layers: &[RecallDebugLayer]) -> RecallDebugPanelSummary {
	let mut summary = RecallDebugPanelSummary { layer_count: layers.len(), ..Default::default() };

	for layer in layers {
		summary.row_count += layer.row_count;
		summary.selected_count += layer.selected_count;
		summary.dropped_count += layer.dropped_count;
		summary.available_count += layer.available_count;

		if layer.evidence_class == "not_requested" {
			summary.not_requested_layer_count += 1;
		}
		if matches!(layer.evidence_class.as_str(), "incomplete" | "blocked" | "wrong_result") {
			summary.incomplete_layer_count += 1;
		}
		if layer.raw_sql_needed {
			summary.raw_sql_needed_count += 1;
		}

		summary.replay_command_count += layer
			.rows
			.iter()
			.filter(|row| row.replay_command.as_ref().is_some_and(|value| !value.is_empty()))
			.count();
		*summary.evidence_class_counts.entry(layer.evidence_class.clone()).or_default() += 1;
	}

	summary
}

fn build_recall_trace(layers: &[RecallDebugLayer]) -> RecallTrace {
	let mut entries = Vec::new();

	for layer in layers {
		if layer.rows.is_empty() {
			if matches!(
				layer.evidence_class.as_str(),
				"blocked" | "not_requested" | "incomplete" | "wrong_result"
			) {
				entries.push(layer_trace_entry(layer));
			}

			continue;
		}

		entries.extend(layer.rows.iter().map(row_trace_entry));
	}

	let summary = summarize_trace_entries(&entries);

	RecallTrace { schema: ELF_RECALL_TRACE_SCHEMA_V1.to_string(), summary, entries }
}

fn summarize_trace_entries(entries: &[RecallTraceEntry]) -> RecallTraceSummary {
	let mut summary = RecallTraceSummary { entry_count: entries.len(), ..Default::default() };

	for entry in entries {
		match entry.selection_state.as_str() {
			"selected" => summary.selected_count += 1,
			"dropped" => summary.dropped_count += 1,
			"blocked" => summary.blocked_count += 1,
			"not_requested" => summary.not_requested_count += 1,
			_ => {},
		}

		if entry.context_state == "stale" || stale_freshness_state(&entry.freshness_state) {
			summary.stale_count += 1;
		}
		if entry.raw_sql_needed {
			summary.raw_sql_needed_count += 1;
		}
		if entry.replay_command.as_ref().is_some_and(|value| !value.is_empty()) {
			summary.replay_command_count += 1;
		}
	}

	summary
}

fn layer_trace_entry(layer: &RecallDebugLayer) -> RecallTraceEntry {
	let context_state = match layer.evidence_class.as_str() {
		"not_requested" => "not_requested",
		"blocked" => "blocked",
		"incomplete" => "incomplete",
		"wrong_result" => "wrong_result",
		_ => "available",
	};

	RecallTraceEntry {
		layer: layer.layer.clone(),
		context_state: context_state.to_string(),
		selection_state: layer.evidence_class.clone(),
		authority_layer: layer.layer.clone(),
		freshness_state: layer.evidence_class.clone(),
		item_ref: serde_json::json!({
			"layer": layer.layer.clone(),
			"anchor": layer.anchor.clone(),
		}),
		source_refs: serde_json::json!([]),
		score: None,
		rank: None,
		policy_reason: Some(layer.summary.clone()),
		replay_command: None,
		evidence_class: layer.evidence_class.clone(),
		raw_sql_needed: layer.raw_sql_needed,
	}
}

fn row_trace_entry(row: &RecallDebugRow) -> RecallTraceEntry {
	let context_state = if stale_freshness_state(&row.freshness_state) {
		"stale"
	} else {
		row.selection_state.as_str()
	};

	RecallTraceEntry {
		layer: row.layer.clone(),
		context_state: context_state.to_string(),
		selection_state: row.selection_state.clone(),
		authority_layer: row.authority_layer.clone(),
		freshness_state: row.freshness_state.clone(),
		item_ref: row.item_ref.clone(),
		source_refs: row.source_refs.clone(),
		score: row.score,
		rank: row.rank,
		policy_reason: row.stage_reason.clone().or_else(|| row.rationale.clone()),
		replay_command: row.replay_command.clone(),
		evidence_class: row.evidence_class.clone(),
		raw_sql_needed: false,
	}
}

fn stale_freshness_state(freshness_state: &str) -> bool {
	matches!(
		freshness_state,
		"stale"
			| "deprecated"
			| "deleted"
			| "superseded"
			| "tombstoned"
			| "historical"
			| "archived"
			| "lint_warning"
			| "lint_error"
	)
}

fn layer_from_rows(
	layer: &str,
	evidence_class: &str,
	anchor: Option<String>,
	summary: &str,
	rows: Vec<RecallDebugRow>,
) -> RecallDebugLayer {
	let selected_count = rows.iter().filter(|row| row.selection_state == "selected").count();
	let dropped_count = rows.iter().filter(|row| row.selection_state == "dropped").count();
	let available_count = rows
		.iter()
		.filter(|row| matches!(row.selection_state.as_str(), "available" | "reviewable"))
		.count();
	let replayable = rows.iter().any(|row| row.replay_command.is_some());

	RecallDebugLayer {
		layer: layer.to_string(),
		evidence_class: evidence_class.to_string(),
		summary: summary.to_string(),
		anchor,
		row_count: rows.len(),
		selected_count,
		dropped_count,
		available_count,
		raw_sql_needed: false,
		replayable,
		rows,
	}
}

fn not_requested_layer(layer: &str, summary: &str) -> RecallDebugLayer {
	RecallDebugLayer {
		layer: layer.to_string(),
		evidence_class: "not_requested".to_string(),
		summary: summary.to_string(),
		anchor: None,
		row_count: 0,
		selected_count: 0,
		dropped_count: 0,
		available_count: 0,
		raw_sql_needed: false,
		replayable: false,
		rows: Vec::new(),
	}
}

fn blocked_layer(
	layer: &str,
	anchor: Option<String>,
	summary: &str,
	err: &Error,
) -> RecallDebugLayer {
	RecallDebugLayer {
		layer: layer.to_string(),
		evidence_class: "blocked".to_string(),
		summary: format!("{summary} error_class={}", public_error_class(err)),
		anchor,
		row_count: 0,
		selected_count: 0,
		dropped_count: 0,
		available_count: 0,
		raw_sql_needed: false,
		replayable: false,
		rows: Vec::new(),
	}
}

fn public_error_class(err: &Error) -> &'static str {
	match err {
		Error::NonEnglishInput { .. } => "validation_non_english_input",
		Error::InvalidRequest { .. } => "validation_invalid_request",
		Error::ScopeDenied { .. } => "scope_denied",
		Error::NotFound { .. } => "not_found",
		Error::Conflict { .. } => "conflict",
		Error::Provider { .. } => "provider_unavailable",
		Error::Storage { .. } => "storage_unavailable",
		Error::Qdrant { .. } => "vector_store_unavailable",
	}
}

fn json_anchor<T>(value: &T) -> Option<String>
where
	T: Serialize + ?Sized,
{
	serde_json::to_value(value).ok().map(|value| value.to_string())
}

fn search_item_candidate_key(item: &SearchExplainItem) -> Option<(Uuid, Uuid)> {
	item.chunk_id.map(|chunk_id| candidate_identity(item.note_id, chunk_id))
}

fn candidate_identity(note_id: Uuid, chunk_id: Uuid) -> (Uuid, Uuid) {
	(note_id, chunk_id)
}

fn candidate_is_selected(
	selected_candidate_keys: &BTreeSet<(Uuid, Uuid)>,
	candidate: &TraceReplayCandidate,
) -> bool {
	selected_candidate_keys.contains(&candidate_identity(candidate.note_id, candidate.chunk_id))
}

fn graph_replay_command(subject: &str, predicate: Option<&GraphQueryPredicateRef>) -> String {
	if let Some(predicate) = predicate.and_then(json_anchor) {
		format!("elf_graph_report subject={subject} predicate={predicate} explain=true")
	} else {
		format!("elf_graph_report subject={subject} explain=true")
	}
}

fn freshness_from_note_source(source: Option<&NoteDebugSourceRow>) -> String {
	source.map(|row| row.status.clone()).unwrap_or_else(|| "unknown".to_string())
}

fn source_ref_from_note_source(source: Option<&NoteDebugSourceRow>) -> Value {
	source.map(|row| serde_json::json!([row.source_ref])).unwrap_or_else(|| serde_json::json!([]))
}

fn last_stage_name(stages: &[SearchTrajectoryStage]) -> Option<String> {
	stages.last().map(|stage| stage.stage_name.clone())
}

fn knowledge_freshness(item: &KnowledgePageSearchItem) -> String {
	if item.lint_summary.error_count > 0 {
		"lint_error".to_string()
	} else if item.lint_summary.warning_count > 0 {
		"lint_warning".to_string()
	} else if item.trust_state != "clean" {
		item.trust_state.clone()
	} else {
		item.status.clone()
	}
}

fn graph_temporal_status(status: crate::RelationTemporalStatus) -> String {
	match status {
		crate::RelationTemporalStatus::Future => "future",
		crate::RelationTemporalStatus::Current => "current",
		crate::RelationTemporalStatus::Historical => "historical",
	}
	.to_string()
}

#[cfg(test)]
mod tests {
	use std::collections::HashSet;

	use time::OffsetDateTime;

	use crate::{
		RecallDebugRow,
		access::SharedSpaceGrantKey,
		recall_debug::{self, BTreeSet, Error, Uuid},
	};
	use elf_storage::models::MemoryNote;

	#[test]
	fn summary_preserves_not_requested_and_replay_counts() {
		let layers = vec![
			recall_debug::not_requested_layer("graph_facts", "missing graph subject"),
			recall_debug::layer_from_rows(
				"memory_notes",
				"pass",
				Some("trace".to_string()),
				"trace rows",
				vec![
					RecallDebugRow {
						layer: "memory_notes".to_string(),
						item_ref: serde_json::json!({"note_id": "n1"}),
						selection_state: "selected".to_string(),
						authority_layer: "memory_note".to_string(),
						freshness_state: "active".to_string(),
						source_refs: serde_json::json!([]),
						score: Some(1.0),
						rank: Some(1),
						rationale: None,
						stage_reason: None,
						replay_command: Some("elf_admin_trace_bundle_get".to_string()),
						evidence_class: "pass".to_string(),
						debug_artifacts: serde_json::json!({}),
					},
					RecallDebugRow {
						layer: "memory_notes".to_string(),
						item_ref: serde_json::json!({"note_id": "n2"}),
						selection_state: "dropped".to_string(),
						authority_layer: "memory_note".to_string(),
						freshness_state: "active".to_string(),
						source_refs: serde_json::json!([]),
						score: Some(0.5),
						rank: Some(2),
						rationale: None,
						stage_reason: Some("not_in_final_top_k".to_string()),
						replay_command: Some("elf_admin_trace_bundle_get".to_string()),
						evidence_class: "pass".to_string(),
						debug_artifacts: serde_json::json!({}),
					},
				],
			),
		];
		let summary = recall_debug::summarize_layers(&layers);

		assert_eq!(summary.layer_count, 2);
		assert_eq!(summary.row_count, 2);
		assert_eq!(summary.selected_count, 1);
		assert_eq!(summary.dropped_count, 1);
		assert_eq!(summary.not_requested_layer_count, 1);
		assert_eq!(summary.replay_command_count, 2);
		assert_eq!(summary.evidence_class_counts.get("pass"), Some(&1));
		assert_eq!(summary.evidence_class_counts.get("not_requested"), Some(&1));
	}

	#[test]
	fn not_requested_layers_never_require_raw_sql() {
		let layer = recall_debug::not_requested_layer("source_documents", "missing query");

		assert_eq!(layer.evidence_class, "not_requested");
		assert_eq!(layer.row_count, 0);
		assert!(!layer.raw_sql_needed);
		assert!(!layer.replayable);
	}

	#[test]
	fn blocked_layers_are_counted_as_incomplete_evidence() {
		let layer = recall_debug::blocked_layer(
			"source_documents",
			Some("alpha".to_string()),
			"docs search failed",
			&Error::Storage { message: "database unavailable".to_string() },
		);
		let summary = recall_debug::summarize_layers(&[layer]);

		assert_eq!(summary.layer_count, 1);
		assert_eq!(summary.incomplete_layer_count, 1);
		assert_eq!(summary.evidence_class_counts.get("blocked"), Some(&1));
	}

	#[test]
	fn blocked_layer_does_not_expose_raw_backend_errors() {
		let layer = recall_debug::blocked_layer(
			"graph_facts",
			None,
			"graph report failed",
			&Error::Storage { message: "password=secret host=db.internal".to_string() },
		);

		assert!(layer.summary.contains("error_class=storage_unavailable"));
		assert!(!layer.summary.contains("password=secret"));
		assert!(!layer.summary.contains("db.internal"));
	}

	#[test]
	fn selected_candidate_filter_is_chunk_level() {
		let note_id = Uuid::new_v4();
		let selected_chunk_id = Uuid::new_v4();
		let dropped_chunk_id = Uuid::new_v4();
		let selected =
			BTreeSet::from([recall_debug::candidate_identity(note_id, selected_chunk_id)]);

		assert!(selected.contains(&recall_debug::candidate_identity(note_id, selected_chunk_id)));
		assert!(!selected.contains(&recall_debug::candidate_identity(note_id, dropped_chunk_id)));
	}

	#[test]
	fn debug_note_readability_preserves_stale_owner_context_only() {
		let allowed_scopes = vec!["agent_private".to_string(), "project_shared".to_string()];
		let shared_grants = HashSet::new();
		let mut note = note_for_debug_visibility("owner-agent", "agent_private", "deprecated");

		assert!(recall_debug::note_debug_read_allowed(
			&note,
			"owner-agent",
			&allowed_scopes,
			&shared_grants
		));
		assert!(!recall_debug::note_debug_read_allowed(
			&note,
			"other-agent",
			&allowed_scopes,
			&shared_grants
		));

		note.scope = "project_shared".to_string();

		assert!(!recall_debug::note_debug_read_allowed(
			&note,
			"other-agent",
			&allowed_scopes,
			&shared_grants
		));

		let shared_grants = HashSet::from([SharedSpaceGrantKey {
			scope: "project_shared".to_string(),
			space_owner_agent_id: "owner-agent".to_string(),
		}]);

		assert!(recall_debug::note_debug_read_allowed(
			&note,
			"other-agent",
			&allowed_scopes,
			&shared_grants
		));
	}

	#[test]
	fn recall_trace_flattens_stale_and_dropped_context() {
		let layers = vec![
			recall_debug::layer_from_rows(
				"memory_notes",
				"pass",
				Some("trace".to_string()),
				"trace rows",
				vec![
					RecallDebugRow {
						layer: "memory_notes".to_string(),
						item_ref: serde_json::json!({"note_id": "selected-stale"}),
						selection_state: "selected".to_string(),
						authority_layer: "memory_note".to_string(),
						freshness_state: "deprecated".to_string(),
						source_refs: serde_json::json!([{"schema": "source_ref/v1"}]),
						score: Some(0.9),
						rank: Some(1),
						rationale: Some("selected but stale".to_string()),
						stage_reason: Some("status=deprecated".to_string()),
						replay_command: Some("elf_trace".to_string()),
						evidence_class: "pass".to_string(),
						debug_artifacts: serde_json::json!({}),
					},
					RecallDebugRow {
						layer: "memory_notes".to_string(),
						item_ref: serde_json::json!({"note_id": "dropped"}),
						selection_state: "dropped".to_string(),
						authority_layer: "memory_note".to_string(),
						freshness_state: "active".to_string(),
						source_refs: serde_json::json!([]),
						score: Some(0.4),
						rank: Some(4),
						rationale: Some("candidate not narrated".to_string()),
						stage_reason: Some("not_in_final_top_k".to_string()),
						replay_command: Some("elf_trace".to_string()),
						evidence_class: "pass".to_string(),
						debug_artifacts: serde_json::json!({}),
					},
				],
			),
			recall_debug::not_requested_layer("graph_facts", "missing graph subject"),
		];
		let trace = recall_debug::build_recall_trace(&layers);

		assert_eq!(trace.schema, "elf.recall_trace/v1");
		assert_eq!(trace.summary.entry_count, 3);
		assert_eq!(trace.summary.selected_count, 1);
		assert_eq!(trace.summary.dropped_count, 1);
		assert_eq!(trace.summary.stale_count, 1);
		assert_eq!(trace.summary.not_requested_count, 1);
		assert_eq!(trace.summary.replay_command_count, 2);
		assert_eq!(trace.entries[0].context_state, "stale");
		assert_eq!(trace.entries[0].policy_reason.as_deref(), Some("status=deprecated"));
		assert_eq!(trace.entries[1].context_state, "dropped");
		assert_eq!(trace.entries[1].policy_reason.as_deref(), Some("not_in_final_top_k"));
		assert_eq!(trace.entries[2].context_state, "not_requested");
	}

	#[test]
	fn recall_trace_counts_blocked_layers_without_backend_details() {
		let layer = recall_debug::blocked_layer(
			"source_documents",
			Some("alpha".to_string()),
			"docs search failed",
			&Error::Storage { message: "password=secret host=db.internal".to_string() },
		);
		let trace = recall_debug::build_recall_trace(&[layer]);

		assert_eq!(trace.summary.blocked_count, 1);
		assert_eq!(trace.entries[0].context_state, "blocked");
		assert_eq!(trace.entries[0].selection_state, "blocked");
		assert!(
			trace.entries[0]
				.policy_reason
				.as_deref()
				.is_some_and(|reason| reason.contains("error_class=storage_unavailable"))
		);
		assert!(
			trace.entries[0]
				.policy_reason
				.as_deref()
				.is_some_and(|reason| !reason.contains("password=secret"))
		);
	}

	fn note_for_debug_visibility(agent_id: &str, scope: &str, status: &str) -> MemoryNote {
		let now = OffsetDateTime::now_utc();

		MemoryNote {
			note_id: Uuid::new_v4(),
			tenant_id: "tenant-a".to_string(),
			project_id: "project-a".to_string(),
			agent_id: agent_id.to_string(),
			scope: scope.to_string(),
			r#type: "fact".to_string(),
			key: None,
			text: "Fact: debug visibility test note.".to_string(),
			importance: 0.7,
			confidence: 0.9,
			status: status.to_string(),
			created_at: now,
			updated_at: now,
			expires_at: None,
			embedding_version: "test:v1".to_string(),
			source_ref: serde_json::json!({"schema": "source_ref/v1"}),
			hit_count: 0,
			last_hit_at: None,
		}
	}
}
