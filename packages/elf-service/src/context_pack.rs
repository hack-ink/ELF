//! Context Pack v1 read-time assembly over recall/debug readbacks.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
	ElfService, Error, GraphQueryEntityRef, GraphQueryPredicateRef, RecallDebugLayer,
	RecallDebugPanelRequest, RecallDebugPanelResponse, RecallDebugRow, RecallTrace, Result,
};
use elf_domain::english_gate;

/// Context Pack v1 schema identifier.
pub const ELF_CONTEXT_PACK_SCHEMA_V1: &str = "elf.context_pack/v1";
/// Context Pack routing trace schema identifier.
pub const ELF_CONTEXT_PACK_ROUTING_TRACE_SCHEMA_V1: &str = "elf.context_pack.routing_trace/v1";

const LAYER_MEMORY: &str = "memory_notes";
const LAYER_DOCS: &str = "source_documents";
const LAYER_KNOWLEDGE: &str = "knowledge_pages";
const LAYER_GRAPH: &str = "graph_facts";
const LAYER_DREAMING: &str = "dreaming_proposals";
const PACK_TTL_MINUTES: i64 = 30;
const DEFAULT_CONTEXT_PACK_LIMIT: u32 = 12;
const MAX_CONTEXT_PACK_LIMIT: u32 = 50;

/// Request payload for a read-time Context Pack.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ContextPackRequest {
	/// Tenant that owns the readback.
	pub tenant_id: String,
	/// Project that owns the readback.
	pub project_id: String,
	/// Agent requesting the readback.
	pub agent_id: String,
	/// Read profile used for every underlying recall surface.
	pub read_profile: String,
	/// Task description used by automatic routing.
	pub task: String,
	/// Optional caller-provided title.
	pub title: Option<String>,
	/// Optional caller-provided description.
	pub description: Option<String>,
	/// Optional search trace anchor for memory rows.
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
	/// Whether to include Dreaming review queue proposals.
	pub include_dreaming: Option<bool>,
	/// Maximum pack items.
	pub limit: Option<u32>,
	/// Debug/test/admin-only routing overrides.
	pub debug_overrides: Option<ContextPackDebugOverrides>,
}

/// Debug/test/admin-only overrides for automatic routing.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ContextPackDebugOverrides {
	/// Layers to force-enable when their required anchors are present.
	#[serde(default)]
	pub enable_layers: Vec<String>,
	/// Layers to suppress for this pack.
	#[serde(default)]
	pub disable_layers: Vec<String>,
	/// Layers to prioritize after normal eligibility checks.
	#[serde(default)]
	pub pin_layers: Vec<String>,
}

/// Read-time Context Pack response.
#[derive(Clone, Debug, Serialize)]
pub struct ContextPackResponse {
	/// Response schema identifier.
	pub schema: String,
	/// Schema version.
	pub version: u32,
	/// Ephemeral pack identifier.
	pub pack_id: Uuid,
	#[serde(with = "crate::time_serde")]
	/// Pack generation timestamp.
	pub generated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Pack expiration timestamp.
	pub expires_at: OffsetDateTime,
	/// Pack title.
	pub title: String,
	/// Pack description.
	pub description: String,
	/// Automatic activation policy and override boundaries.
	pub activation_policy: ContextPackActivationPolicy,
	/// Authority layers considered by this pack.
	pub authority_layers: Vec<ContextPackAuthorityLayer>,
	/// Typed selectors per layer.
	pub typed_selectors: BTreeMap<String, ContextPackLayerSelector>,
	/// Required anchors and whether they were supplied.
	pub required_anchors: Vec<ContextPackRequiredAnchor>,
	/// Read-profile policy applied by underlying recall surfaces.
	pub read_profile_policy: ContextPackReadProfilePolicy,
	/// Freshness policy applied to pack items.
	pub freshness_policy: ContextPackFreshnessPolicy,
	/// Budget limits for the pack.
	pub budget_limits: ContextPackBudgetLimits,
	/// Ranking policy for item ordering.
	pub ranking_policy: ContextPackRankingPolicy,
	/// Debug and privacy policy.
	pub debug_policy: ContextPackDebugPolicy,
	/// Activation and selection trace.
	pub routing_trace: ContextPackRoutingTrace,
	/// Bounded eligible context item references.
	pub items: Vec<ContextPackItem>,
	/// Underlying recall trace after scope and freshness gates.
	pub recall_trace: RecallTrace,
}

/// Automatic activation policy metadata.
#[derive(Clone, Debug, Serialize)]
pub struct ContextPackActivationPolicy {
	/// Policy schema identifier.
	pub schema: String,
	/// Routing mode.
	pub mode: String,
	/// Whether manual overrides were supplied.
	pub manual_override_present: bool,
	/// Override boundary.
	pub override_policy: String,
}

/// Authority layer metadata.
#[derive(Clone, Debug, Serialize)]
pub struct ContextPackAuthorityLayer {
	/// Layer identifier.
	pub layer: String,
	/// Authority class for the layer.
	pub authority_state: String,
	/// Whether selected rows from this layer can become pack items.
	pub eligible_for_pack_items: bool,
}

/// Layer selector state.
#[derive(Clone, Debug, Serialize)]
pub struct ContextPackLayerSelector {
	/// Layer identifier.
	pub layer: String,
	/// Selector state.
	pub state: String,
	/// Selector kind.
	pub selector_type: String,
	/// Selector payload.
	pub selector: Value,
	/// Reason code.
	pub reason_code: String,
	/// Whether this selector was affected by a manual override.
	pub manual_override: bool,
	/// Whether this layer was pinned for priority.
	pub pinned: bool,
}

/// Required anchor state.
#[derive(Clone, Debug, Serialize)]
pub struct ContextPackRequiredAnchor {
	/// Layer identifier.
	pub layer: String,
	/// Required anchor name.
	pub anchor: String,
	/// Whether the anchor was supplied.
	pub supplied: bool,
	/// Reason code for missing or present state.
	pub reason_code: String,
}

/// Read-profile policy metadata.
#[derive(Clone, Debug, Serialize)]
pub struct ContextPackReadProfilePolicy {
	/// Read profile used for all layers.
	pub read_profile: String,
	/// Privacy boundary.
	pub boundary: String,
	/// Scope behavior.
	pub scope_policy: String,
}

/// Freshness policy metadata.
#[derive(Clone, Debug, Serialize)]
pub struct ContextPackFreshnessPolicy {
	/// Current-source requirement.
	pub current_only: bool,
	/// Suppressed freshness states.
	pub suppressed_states: Vec<String>,
	/// Redaction behavior.
	pub redaction_policy: String,
}

/// Pack budget limits.
#[derive(Clone, Debug, Serialize)]
pub struct ContextPackBudgetLimits {
	/// Maximum returned items.
	pub max_items: u32,
	/// Maximum rows requested per recall layer.
	pub per_layer_recall_limit: u32,
}

/// Pack ranking policy.
#[derive(Clone, Debug, Serialize)]
pub struct ContextPackRankingPolicy {
	/// Ranking schema identifier.
	pub schema: String,
	/// Priority behavior.
	pub priority_policy: String,
	/// Whether pinning can bypass eligibility.
	pub pin_bypasses_eligibility: bool,
}

/// Pack debug policy.
#[derive(Clone, Debug, Serialize)]
pub struct ContextPackDebugPolicy {
	/// Debug schema identifier.
	pub schema: String,
	/// Whether activation trace is included.
	pub activation_trace: bool,
	/// Source-ref privacy policy.
	pub source_ref_policy: String,
	/// Unreadable evidence policy.
	pub unreadable_policy: String,
}

/// Context Pack routing trace.
#[derive(Clone, Debug, Serialize)]
pub struct ContextPackRoutingTrace {
	/// Trace schema identifier.
	pub schema: String,
	/// Trace entries.
	pub entries: Vec<ContextPackRoutingTraceEntry>,
	/// Selected layer count.
	pub selected_count: usize,
	/// Suppressed layer count.
	pub suppressed_count: usize,
	/// Blocked layer count.
	pub blocked_count: usize,
	/// Not-requested layer count.
	pub not_requested_count: usize,
	/// Pinned layers that remained ineligible.
	pub pinned_ineligible_count: usize,
}

/// One routing trace entry.
#[derive(Clone, Debug, Serialize)]
pub struct ContextPackRoutingTraceEntry {
	/// Layer identifier.
	pub layer: String,
	/// Activation state.
	pub activation_state: String,
	/// Reason code.
	pub reason_code: String,
	/// Human-readable policy reason.
	pub policy_reason: String,
	/// Whether a manual override affected the layer.
	pub manual_override: bool,
	/// Whether the layer was pinned.
	pub pinned: bool,
	/// Privacy note.
	pub privacy: String,
}

/// One item reference in the Context Pack.
#[derive(Clone, Debug, Serialize)]
pub struct ContextPackItem {
	/// Source layer.
	pub layer: String,
	/// Authority layer.
	pub authority_layer: String,
	/// Freshness state.
	pub freshness_state: String,
	/// Item reference.
	pub item_ref: Value,
	/// Source refs for current readable evidence only.
	pub source_refs: Value,
	/// Score if available.
	pub score: Option<f32>,
	/// Rank if available.
	pub rank: Option<u32>,
	/// Selection or routing reason.
	pub reason_code: String,
	/// Whether pinning raised this item's priority.
	pub pinned_priority: bool,
}

#[derive(Clone, Debug)]
struct RoutedPack {
	limit: u32,
	title: String,
	description: String,
	docs_query: Option<String>,
	knowledge_query: Option<String>,
	include_dreaming: bool,
	selectors: BTreeMap<String, ContextPackLayerSelector>,
	required_anchors: Vec<ContextPackRequiredAnchor>,
	disabled_layers: BTreeSet<String>,
	pinned_layers: BTreeSet<String>,
}

struct LayerRouteInput<'a> {
	layer: &'a str,
	default_enabled: bool,
	manual_enabled: bool,
	manual_disabled: bool,
	pinned: bool,
	anchor_name: &'a str,
	anchor_supplied: bool,
	selector_type: &'a str,
	selector: Value,
}

struct SelectorSources<'a> {
	req: &'a ContextPackRequest,
	docs_query: &'a Option<String>,
	knowledge_query: &'a Option<String>,
	include_dreaming: bool,
	enabled: &'a BTreeSet<String>,
	disabled: &'a BTreeSet<String>,
	pinned: &'a BTreeSet<String>,
}

impl ElfService {
	/// Builds a Context Pack as an ephemeral read-time view over current recall layers.
	pub async fn context_pack_build(&self, req: ContextPackRequest) -> Result<ContextPackResponse> {
		validate_context_pack_request(&req)?;

		let routed = route_context_pack(&req);
		let recall = self
			.recall_debug_panel(RecallDebugPanelRequest {
				tenant_id: req.tenant_id.clone(),
				project_id: req.project_id.clone(),
				agent_id: req.agent_id.clone(),
				read_profile: req.read_profile.clone(),
				trace_id: selector_enabled(&routed, LAYER_MEMORY).then_some(req.trace_id).flatten(),
				query: None,
				docs_query: selector_enabled(&routed, LAYER_DOCS)
					.then(|| routed.docs_query.clone())
					.flatten(),
				knowledge_query: selector_enabled(&routed, LAYER_KNOWLEDGE)
					.then(|| routed.knowledge_query.clone())
					.flatten(),
				graph_subject: selector_enabled(&routed, LAYER_GRAPH)
					.then(|| req.graph_subject.clone())
					.flatten(),
				graph_predicate: selector_enabled(&routed, LAYER_GRAPH)
					.then(|| req.graph_predicate.clone())
					.flatten(),
				include_dreaming: Some(
					selector_enabled(&routed, LAYER_DREAMING) && routed.include_dreaming,
				),
				limit: Some(routed.limit),
				allow_project_trace_debug: false,
			})
			.await?;

		Ok(build_context_pack_response(&req, routed, recall))
	}
}

fn validate_context_pack_request(req: &ContextPackRequest) -> Result<()> {
	validate_required_english("task", req.task.as_str())?;
	validate_optional_english("title", req.title.as_deref())?;
	validate_optional_english("description", req.description.as_deref())?;
	validate_optional_english("query", req.query.as_deref())?;
	validate_optional_english("docs_query", req.docs_query.as_deref())?;
	validate_optional_english("knowledge_query", req.knowledge_query.as_deref())?;
	validate_graph_entity_ref("graph_subject", req.graph_subject.as_ref())?;
	validate_graph_predicate_ref("graph_predicate", req.graph_predicate.as_ref())?;

	Ok(())
}

fn validate_required_english(field: &str, value: &str) -> Result<()> {
	let trimmed = value.trim();

	if trimmed.is_empty() {
		return Err(Error::InvalidRequest { message: format!("{field} must be non-empty.") });
	}
	if !english_gate::is_english_natural_language(trimmed) {
		return Err(Error::NonEnglishInput { field: format!("$.{field}") });
	}

	Ok(())
}

fn validate_optional_english(field: &str, value: Option<&str>) -> Result<()> {
	if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty())
		&& !english_gate::is_english_natural_language(value)
	{
		return Err(Error::NonEnglishInput { field: format!("$.{field}") });
	}

	Ok(())
}

fn validate_graph_entity_ref(field: &str, value: Option<&GraphQueryEntityRef>) -> Result<()> {
	if let Some(GraphQueryEntityRef::Surface { surface }) = value {
		validate_identifier_english(format!("$.{field}.surface"), surface.as_str())?;
	}

	Ok(())
}

fn validate_graph_predicate_ref(field: &str, value: Option<&GraphQueryPredicateRef>) -> Result<()> {
	if let Some(GraphQueryPredicateRef::Surface { surface }) = value {
		validate_identifier_english(format!("$.{field}.surface"), surface.as_str())?;
	}

	Ok(())
}

fn validate_identifier_english(field: String, value: &str) -> Result<()> {
	if !english_gate::is_english_identifier(value.trim()) {
		return Err(Error::NonEnglishInput { field });
	}

	Ok(())
}

fn route_context_pack(req: &ContextPackRequest) -> RoutedPack {
	let limit = req.limit.unwrap_or(DEFAULT_CONTEXT_PACK_LIMIT).clamp(1, MAX_CONTEXT_PACK_LIMIT);
	let task = trimmed_opt(Some(req.task.as_str()));
	let base_query = trimmed_opt(req.query.as_deref()).or_else(|| task.clone());
	let docs_query = trimmed_opt(req.docs_query.as_deref()).or_else(|| base_query.clone());
	let knowledge_query =
		trimmed_opt(req.knowledge_query.as_deref()).or_else(|| base_query.clone());
	let include_dreaming = req.include_dreaming == Some(true)
		|| task.as_deref().is_some_and(|value| {
			contains_any(value, &["proposal", "review", "dreaming", "consolidation"])
		});
	let overrides = req.debug_overrides.clone().unwrap_or_default();
	let enabled = normalize_layers(overrides.enable_layers);
	let disabled = normalize_layers(overrides.disable_layers);
	let pinned = normalize_layers(overrides.pin_layers);
	let title = trimmed_opt(req.title.as_deref()).unwrap_or_else(|| "Context Pack".to_string());
	let description = trimmed_opt(req.description.as_deref()).unwrap_or_else(|| {
		"Read-time scoped context assembled from current ELF authority layers.".to_string()
	});
	let (selectors, required_anchors) = route_selectors(SelectorSources {
		req,
		docs_query: &docs_query,
		knowledge_query: &knowledge_query,
		include_dreaming,
		enabled: &enabled,
		disabled: &disabled,
		pinned: &pinned,
	});

	RoutedPack {
		limit,
		title,
		description,
		docs_query,
		knowledge_query,
		include_dreaming: include_dreaming || enabled.contains(LAYER_DREAMING),
		selectors,
		required_anchors,
		disabled_layers: disabled,
		pinned_layers: pinned,
	}
}

fn route_selectors(
	input: SelectorSources<'_>,
) -> (BTreeMap<String, ContextPackLayerSelector>, Vec<ContextPackRequiredAnchor>) {
	let mut selectors = BTreeMap::new();
	let mut required_anchors = Vec::new();

	add_selector(
		&mut selectors,
		&mut required_anchors,
		LayerRouteInput {
			layer: LAYER_MEMORY,
			default_enabled: input.req.trace_id.is_some(),
			manual_enabled: input.enabled.contains(LAYER_MEMORY),
			manual_disabled: input.disabled.contains(LAYER_MEMORY),
			pinned: input.pinned.contains(LAYER_MEMORY),
			anchor_name: "trace_id",
			anchor_supplied: input.req.trace_id.is_some(),
			selector_type: "trace",
			selector: input
				.req
				.trace_id
				.map(|trace_id| serde_json::json!({ "trace_id": trace_id }))
				.unwrap_or_else(|| serde_json::json!({})),
		},
	);
	add_selector(
		&mut selectors,
		&mut required_anchors,
		LayerRouteInput {
			layer: LAYER_DOCS,
			default_enabled: input.docs_query.is_some(),
			manual_enabled: input.enabled.contains(LAYER_DOCS),
			manual_disabled: input.disabled.contains(LAYER_DOCS),
			pinned: input.pinned.contains(LAYER_DOCS),
			anchor_name: "docs_query",
			anchor_supplied: input.docs_query.is_some(),
			selector_type: "query",
			selector: input
				.docs_query
				.as_ref()
				.map(|query| serde_json::json!({ "query": query }))
				.unwrap_or_else(|| serde_json::json!({})),
		},
	);
	add_selector(
		&mut selectors,
		&mut required_anchors,
		LayerRouteInput {
			layer: LAYER_KNOWLEDGE,
			default_enabled: input.knowledge_query.is_some(),
			manual_enabled: input.enabled.contains(LAYER_KNOWLEDGE),
			manual_disabled: input.disabled.contains(LAYER_KNOWLEDGE),
			pinned: input.pinned.contains(LAYER_KNOWLEDGE),
			anchor_name: "knowledge_query",
			anchor_supplied: input.knowledge_query.is_some(),
			selector_type: "query",
			selector: input
				.knowledge_query
				.as_ref()
				.map(|query| serde_json::json!({ "query": query }))
				.unwrap_or_else(|| serde_json::json!({})),
		},
	);
	add_selector(
		&mut selectors,
		&mut required_anchors,
		LayerRouteInput {
			layer: LAYER_GRAPH,
			default_enabled: input.req.graph_subject.is_some(),
			manual_enabled: input.enabled.contains(LAYER_GRAPH),
			manual_disabled: input.disabled.contains(LAYER_GRAPH),
			pinned: input.pinned.contains(LAYER_GRAPH),
			anchor_name: "graph_subject",
			anchor_supplied: input.req.graph_subject.is_some(),
			selector_type: "graph_subject",
			selector: input
				.req
				.graph_subject
				.as_ref()
				.map(|subject| serde_json::json!({ "subject": subject }))
				.unwrap_or_else(|| serde_json::json!({})),
		},
	);
	add_selector(
		&mut selectors,
		&mut required_anchors,
		LayerRouteInput {
			layer: LAYER_DREAMING,
			default_enabled: input.include_dreaming,
			manual_enabled: input.enabled.contains(LAYER_DREAMING),
			manual_disabled: input.disabled.contains(LAYER_DREAMING),
			pinned: input.pinned.contains(LAYER_DREAMING),
			anchor_name: "include_dreaming",
			anchor_supplied: input.include_dreaming || input.enabled.contains(LAYER_DREAMING),
			selector_type: "review_queue",
			selector: serde_json::json!({
				"include_dreaming": input.include_dreaming || input.enabled.contains(LAYER_DREAMING)
			}),
		},
	);

	(selectors, required_anchors)
}

fn add_selector(
	selectors: &mut BTreeMap<String, ContextPackLayerSelector>,
	required_anchors: &mut Vec<ContextPackRequiredAnchor>,
	input: LayerRouteInput<'_>,
) {
	let manual_override = input.manual_enabled || input.manual_disabled || input.pinned;
	let (state, reason_code) = if input.manual_disabled {
		("suppressed", "MANUAL_DISABLED")
	} else if !input.anchor_supplied && (input.manual_enabled || input.pinned) {
		("suppressed", "PINNED_OR_ENABLED_MISSING_REQUIRED_ANCHOR")
	} else if input.default_enabled || input.manual_enabled {
		("selected", "AUTOMATIC_ROUTING_MATCH")
	} else {
		("not_requested", "AUTOMATIC_ROUTING_NO_MATCH")
	};

	required_anchors.push(ContextPackRequiredAnchor {
		layer: input.layer.to_string(),
		anchor: input.anchor_name.to_string(),
		supplied: input.anchor_supplied,
		reason_code: if input.anchor_supplied { "ANCHOR_SUPPLIED" } else { "ANCHOR_MISSING" }
			.to_string(),
	});
	selectors.insert(
		input.layer.to_string(),
		ContextPackLayerSelector {
			layer: input.layer.to_string(),
			state: state.to_string(),
			selector_type: input.selector_type.to_string(),
			selector: input.selector,
			reason_code: reason_code.to_string(),
			manual_override,
			pinned: input.pinned,
		},
	);
}

fn build_context_pack_response(
	req: &ContextPackRequest,
	routed: RoutedPack,
	recall: RecallDebugPanelResponse,
) -> ContextPackResponse {
	let generated_at = OffsetDateTime::now_utc();
	let mut items = pack_items(&recall.layers, &routed);

	items.truncate(routed.limit as usize);

	let routing_trace = build_routing_trace(&routed, &recall.layers);

	ContextPackResponse {
		schema: ELF_CONTEXT_PACK_SCHEMA_V1.to_string(),
		version: 1,
		pack_id: Uuid::new_v4(),
		generated_at,
		expires_at: generated_at + Duration::minutes(PACK_TTL_MINUTES),
		title: routed.title.clone(),
		description: routed.description.clone(),
		activation_policy: ContextPackActivationPolicy {
			schema: "elf.context_pack.activation_policy/v1".to_string(),
			mode: "automatic".to_string(),
			manual_override_present: req.debug_overrides.is_some(),
			override_policy:
				"manual enable/disable/pin is debug/test/admin-only; pinning changes priority only"
					.to_string(),
		},
		authority_layers: authority_layers(),
		typed_selectors: routed.selectors.clone(),
		required_anchors: routed.required_anchors.clone(),
		read_profile_policy: ContextPackReadProfilePolicy {
			read_profile: req.read_profile.clone(),
			boundary: "all layer reads use the request read_profile and service grants".to_string(),
			scope_policy:
				"agent_private requires owner match; shared scopes require readable grants"
					.to_string(),
		},
		freshness_policy: ContextPackFreshnessPolicy {
			current_only: true,
			suppressed_states: vec![
				"deleted".to_string(),
				"deprecated".to_string(),
				"expired".to_string(),
				"stale".to_string(),
				"superseded".to_string(),
				"tombstoned".to_string(),
			],
			redaction_policy:
				"items include only source refs already returned by readable current recall rows"
					.to_string(),
		},
		budget_limits: ContextPackBudgetLimits {
			max_items: routed.limit,
			per_layer_recall_limit: routed.limit,
		},
		ranking_policy: ContextPackRankingPolicy {
			schema: "elf.context_pack.ranking_policy/v1".to_string(),
			priority_policy:
				"eligible pinned layers sort ahead of unpinned layers, then by rank and score"
					.to_string(),
			pin_bypasses_eligibility: false,
		},
		debug_policy: ContextPackDebugPolicy {
			schema: "elf.context_pack.debug_policy/v1".to_string(),
			activation_trace: true,
			source_ref_policy:
				"debug output omits unreadable/private source existence, counts, refs, and content"
					.to_string(),
			unreadable_policy:
				"unreadable rows are filtered by underlying recall surfaces before pack assembly"
					.to_string(),
		},
		routing_trace,
		items,
		recall_trace: recall.recall_trace,
	}
}

fn authority_layers() -> Vec<ContextPackAuthorityLayer> {
	[
		(LAYER_MEMORY, "authoritative_memory", true),
		(LAYER_DOCS, "source_evidence", true),
		(LAYER_KNOWLEDGE, "derived_knowledge", true),
		(LAYER_GRAPH, "graph_lite_fact", true),
		(LAYER_DREAMING, "reviewable_proposal", true),
	]
	.into_iter()
	.map(|(layer, authority_state, eligible_for_pack_items)| ContextPackAuthorityLayer {
		layer: layer.to_string(),
		authority_state: authority_state.to_string(),
		eligible_for_pack_items,
	})
	.collect()
}

fn build_routing_trace(
	routed: &RoutedPack,
	layers: &[RecallDebugLayer],
) -> ContextPackRoutingTrace {
	let layer_map =
		layers.iter().map(|layer| (layer.layer.as_str(), layer)).collect::<BTreeMap<_, _>>();
	let mut entries = Vec::new();

	for (layer, selector) in &routed.selectors {
		let mut activation_state = selector.state.clone();
		let mut reason_code = selector.reason_code.clone();
		let mut policy_reason = match selector.reason_code.as_str() {
			"MANUAL_DISABLED" => "Layer was suppressed by a debug override.".to_string(),
			"PINNED_OR_ENABLED_MISSING_REQUIRED_ANCHOR" =>
				"Pinned or enabled layer stayed ineligible because a required anchor was missing."
					.to_string(),
			"AUTOMATIC_ROUTING_MATCH" => "Automatic routing selected this layer.".to_string(),
			_ => "Automatic routing did not request this layer.".to_string(),
		};

		if let Some(recall_layer) = layer_map.get(layer.as_str()) {
			if recall_layer.evidence_class == "blocked" {
				activation_state = "blocked".to_string();
				reason_code = "RECALL_LAYER_BLOCKED".to_string();
				policy_reason = recall_layer.summary.clone();
			} else if selector.state == "selected"
				&& recall_layer.rows.iter().all(|row| !row_eligible_for_pack(row))
			{
				activation_state = if selector.pinned {
					"pinned_ineligible".to_string()
				} else {
					"suppressed".to_string()
				};
				reason_code = "NO_CURRENT_READABLE_SELECTED_ROWS".to_string();
				policy_reason =
					"Layer returned no current readable selected, available, or reviewable rows."
						.to_string();
			}
		}

		entries.push(ContextPackRoutingTraceEntry {
			layer: layer.clone(),
			activation_state,
			reason_code,
			policy_reason,
			manual_override: selector.manual_override,
			pinned: selector.pinned,
			privacy: "no unreadable source existence, counts, refs, or content disclosed"
				.to_string(),
		});
	}

	ContextPackRoutingTrace {
		schema: ELF_CONTEXT_PACK_ROUTING_TRACE_SCHEMA_V1.to_string(),
		selected_count: entries.iter().filter(|entry| entry.activation_state == "selected").count(),
		suppressed_count: entries
			.iter()
			.filter(|entry| entry.activation_state == "suppressed")
			.count(),
		blocked_count: entries.iter().filter(|entry| entry.activation_state == "blocked").count(),
		not_requested_count: entries
			.iter()
			.filter(|entry| entry.activation_state == "not_requested")
			.count(),
		pinned_ineligible_count: entries
			.iter()
			.filter(|entry| entry.activation_state == "pinned_ineligible")
			.count(),
		entries,
	}
}

fn pack_items(layers: &[RecallDebugLayer], routed: &RoutedPack) -> Vec<ContextPackItem> {
	let mut items = layers
		.iter()
		.filter(|layer| !routed.disabled_layers.contains(layer.layer.as_str()))
		.flat_map(|layer| {
			layer.rows.iter().filter(|row| row_eligible_for_pack(row)).map(|row| {
				let pinned_priority = routed.pinned_layers.contains(row.layer.as_str());

				ContextPackItem {
					layer: row.layer.clone(),
					authority_layer: row.authority_layer.clone(),
					freshness_state: row.freshness_state.clone(),
					item_ref: row.item_ref.clone(),
					source_refs: row.source_refs.clone(),
					score: row.score,
					rank: row.rank,
					reason_code: row
						.stage_reason
						.clone()
						.or_else(|| row.rationale.clone())
						.unwrap_or_else(|| "selected_by_recall_layer".to_string()),
					pinned_priority,
				}
			})
		})
		.collect::<Vec<_>>();

	items.sort_by(|left, right| {
		right
			.pinned_priority
			.cmp(&left.pinned_priority)
			.then_with(|| left.rank.unwrap_or(u32::MAX).cmp(&right.rank.unwrap_or(u32::MAX)))
			.then_with(|| {
				right
					.score
					.unwrap_or(f32::NEG_INFINITY)
					.total_cmp(&left.score.unwrap_or(f32::NEG_INFINITY))
			})
			.then_with(|| left.layer.cmp(&right.layer))
	});

	items
}

fn row_eligible_for_pack(row: &RecallDebugRow) -> bool {
	matches!(row.selection_state.as_str(), "selected" | "available" | "reviewable")
		&& row.evidence_class == "pass"
		&& !stale_or_non_current(row.freshness_state.as_str())
		&& source_refs_present(&row.source_refs)
}

fn stale_or_non_current(freshness_state: &str) -> bool {
	matches!(
		freshness_state,
		"deleted" | "deprecated" | "expired" | "stale" | "superseded" | "tombstoned" | "historical"
	)
}

fn source_refs_present(value: &Value) -> bool {
	match value {
		Value::Null => false,
		Value::Array(values) => !values.is_empty(),
		Value::Object(values) => ["source_refs", "source_ref", "source_snapshot", "affected_refs"]
			.iter()
			.filter_map(|key| values.get(*key))
			.any(source_refs_present),
		Value::String(value) => !value.trim().is_empty(),
		_ => true,
	}
}

fn selector_enabled(routed: &RoutedPack, layer: &str) -> bool {
	routed.selectors.get(layer).is_some_and(|selector| selector.state == "selected")
}

fn trimmed_opt(value: Option<&str>) -> Option<String> {
	value.map(str::trim).filter(|value| !value.is_empty()).map(str::to_string)
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
	let lower = value.to_ascii_lowercase();

	needles.iter().any(|needle| lower.contains(needle))
}

fn normalize_layers(layers: Vec<String>) -> BTreeSet<String> {
	layers
		.into_iter()
		.filter_map(|layer| {
			let normalized = layer.trim().to_ascii_lowercase().replace('-', "_");

			match normalized.as_str() {
				LAYER_MEMORY | "memory" | "notes" => Some(LAYER_MEMORY.to_string()),
				LAYER_DOCS | "docs" | "documents" | "source" => Some(LAYER_DOCS.to_string()),
				LAYER_KNOWLEDGE | "knowledge" | "pages" => Some(LAYER_KNOWLEDGE.to_string()),
				LAYER_GRAPH | "graph" | "relations" => Some(LAYER_GRAPH.to_string()),
				LAYER_DREAMING | "dreaming" | "proposals" => Some(LAYER_DREAMING.to_string()),
				_ => None,
			}
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use serde_json::Value;

	use crate::{
		Error, GraphQueryEntityRef, RecallDebugLayer, RecallDebugRow,
		context_pack::{
			self, ContextPackDebugOverrides, ContextPackRequest, LAYER_DOCS, LAYER_KNOWLEDGE,
			LAYER_MEMORY,
		},
	};

	fn base_request() -> ContextPackRequest {
		ContextPackRequest {
			tenant_id: "tenant".to_string(),
			project_id: "project".to_string(),
			agent_id: "agent".to_string(),
			read_profile: "private_plus_project".to_string(),
			task: "Find current source-backed decisions for routing work.".to_string(),
			title: None,
			description: None,
			trace_id: None,
			query: None,
			docs_query: None,
			knowledge_query: None,
			graph_subject: None,
			graph_predicate: None,
			include_dreaming: None,
			limit: Some(5),
			debug_overrides: None,
		}
	}

	fn row(
		layer: &str,
		selection_state: &str,
		freshness_state: &str,
		source_refs: Value,
	) -> RecallDebugRow {
		RecallDebugRow {
			layer: layer.to_string(),
			item_ref: serde_json::json!({"id": layer}),
			selection_state: selection_state.to_string(),
			authority_layer: layer.to_string(),
			freshness_state: freshness_state.to_string(),
			source_refs,
			score: Some(0.7),
			rank: Some(1),
			rationale: Some("test row".to_string()),
			stage_reason: Some("test_stage".to_string()),
			replay_command: None,
			evidence_class: "pass".to_string(),
			debug_artifacts: serde_json::json!({}),
		}
	}

	#[test]
	fn automatic_routing_activates_query_layers_and_schema_fields() {
		let routed = context_pack::route_context_pack(&base_request());

		assert_eq!(routed.selectors[LAYER_DOCS].state, "selected");
		assert_eq!(routed.selectors[LAYER_KNOWLEDGE].state, "selected");
		assert_eq!(routed.selectors[LAYER_MEMORY].state, "not_requested");
		assert_eq!(routed.limit, 5);
		assert!(routed.required_anchors.iter().any(|anchor| {
			anchor.layer == LAYER_DOCS && anchor.anchor == "docs_query" && anchor.supplied
		}));
	}

	#[test]
	fn manual_disable_suppresses_layer_without_removing_other_automatic_routes() {
		let mut req = base_request();

		req.debug_overrides = Some(ContextPackDebugOverrides {
			disable_layers: vec![LAYER_DOCS.to_string()],
			..ContextPackDebugOverrides::default()
		});

		let routed = context_pack::route_context_pack(&req);

		assert_eq!(routed.selectors[LAYER_DOCS].state, "suppressed");
		assert_eq!(routed.selectors[LAYER_DOCS].reason_code, "MANUAL_DISABLED");
		assert_eq!(routed.selectors[LAYER_KNOWLEDGE].state, "selected");
	}

	#[test]
	fn pinned_missing_anchor_is_ineligible_and_cannot_bypass_requirements() {
		let mut req = base_request();

		req.debug_overrides = Some(ContextPackDebugOverrides {
			pin_layers: vec![LAYER_MEMORY.to_string()],
			..ContextPackDebugOverrides::default()
		});

		let routed = context_pack::route_context_pack(&req);

		assert_eq!(routed.selectors[LAYER_MEMORY].state, "suppressed");
		assert_eq!(
			routed.selectors[LAYER_MEMORY].reason_code,
			"PINNED_OR_ENABLED_MISSING_REQUIRED_ANCHOR"
		);
		assert!(routed.selectors[LAYER_MEMORY].pinned);
	}

	#[test]
	fn pack_items_filter_unreadable_empty_source_refs_and_stale_or_deleted_rows() {
		let rows = vec![
			row(LAYER_DOCS, "selected", "active", serde_json::json!([{"schema": "source_ref/v1"}])),
			row(
				LAYER_DOCS,
				"selected",
				"deleted",
				serde_json::json!([{"schema": "source_ref/v1"}]),
			),
			row(
				LAYER_DOCS,
				"selected",
				"deprecated",
				serde_json::json!([{"schema": "source_ref/v1"}]),
			),
			row(LAYER_DOCS, "selected", "active", serde_json::json!([])),
			row(LAYER_KNOWLEDGE, "selected", "active", serde_json::json!({"source_refs": []})),
			row(LAYER_DOCS, "dropped", "active", serde_json::json!([{"schema": "source_ref/v1"}])),
		];
		let layer = RecallDebugLayer {
			layer: LAYER_DOCS.to_string(),
			evidence_class: "pass".to_string(),
			summary: "docs".to_string(),
			anchor: Some("query".to_string()),
			row_count: rows.len(),
			selected_count: 4,
			dropped_count: 1,
			available_count: 0,
			raw_sql_needed: false,
			replayable: false,
			debug_artifacts: serde_json::json!({}),
			rows,
		};
		let routed = context_pack::route_context_pack(&base_request());
		let items = context_pack::pack_items(&[layer], &routed);

		assert_eq!(items.len(), 1);
		assert_eq!(items[0].freshness_state, "active");
	}

	#[test]
	fn validation_rejects_empty_or_non_english_task_and_query() {
		let mut req = base_request();

		req.task = "   ".to_string();

		assert!(matches!(
			context_pack::validate_context_pack_request(&req),
			Err(Error::InvalidRequest { .. })
		));

		req.task = "Find current source-backed decisions.".to_string();
		req.query = Some("決定".to_string());

		assert!(matches!(
			context_pack::validate_context_pack_request(&req),
			Err(Error::NonEnglishInput { field }) if field == "$.query"
		));

		req.query = None;
		req.title = Some("決定".to_string());

		assert!(matches!(
			context_pack::validate_context_pack_request(&req),
			Err(Error::NonEnglishInput { field }) if field == "$.title"
		));

		req.title = None;
		req.graph_subject = Some(GraphQueryEntityRef::Surface { surface: "決定".to_string() });

		assert!(matches!(
			context_pack::validate_context_pack_request(&req),
			Err(Error::NonEnglishInput { field }) if field == "$.graph_subject.surface"
		));
	}

	#[test]
	fn disabled_layer_rows_are_suppressed_even_when_readable() {
		let mut req = base_request();

		req.debug_overrides = Some(ContextPackDebugOverrides {
			disable_layers: vec![LAYER_DOCS.to_string()],
			..ContextPackDebugOverrides::default()
		});

		let routed = context_pack::route_context_pack(&req);
		let layer = RecallDebugLayer {
			layer: LAYER_DOCS.to_string(),
			evidence_class: "pass".to_string(),
			summary: "docs".to_string(),
			anchor: Some("query".to_string()),
			row_count: 1,
			selected_count: 1,
			dropped_count: 0,
			available_count: 0,
			raw_sql_needed: false,
			replayable: false,
			debug_artifacts: serde_json::json!({}),
			rows: vec![row(
				LAYER_DOCS,
				"selected",
				"active",
				serde_json::json!([{"schema": "source_ref/v1"}]),
			)],
		};

		assert!(context_pack::pack_items(&[layer], &routed).is_empty());
	}

	#[test]
	fn routing_trace_does_not_disclose_suppressed_layer_counts_or_refs() {
		let mut req = base_request();

		req.debug_overrides = Some(ContextPackDebugOverrides {
			pin_layers: vec![LAYER_MEMORY.to_string()],
			..ContextPackDebugOverrides::default()
		});

		let routed = context_pack::route_context_pack(&req);
		let trace = context_pack::build_routing_trace(&routed, &[]);
		let memory = trace
			.entries
			.iter()
			.find(|entry| entry.layer == LAYER_MEMORY)
			.expect("memory trace entry");

		assert_eq!(memory.activation_state, "suppressed");
		assert_eq!(memory.reason_code, "PINNED_OR_ENABLED_MISSING_REQUIRED_ANCHOR");
		assert!(memory.privacy.contains("no unreadable source existence"));
		assert!(!serde_json::to_string(memory).unwrap().contains("source_refs"));
	}
}
