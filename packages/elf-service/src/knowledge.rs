//! Deterministic derived knowledge page rebuild and readback service APIs.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{self, Map, Number, Value};
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	ElfService, Error, Result, access,
	consolidation::{
		ConsolidationProposalInput, ConsolidationRunCreateRequest, ConsolidationRunCreateResponse,
	},
	search,
};
use elf_domain::{
	consolidation::{
		ConsolidationApplyIntent, ConsolidationInputRef, ConsolidationLineage, ConsolidationMarker,
		ConsolidationMarkerSeverity, ConsolidationMarkers, ConsolidationProposalDiff,
		ConsolidationSourceKind, ConsolidationSourceSnapshot,
	},
	english_gate,
	knowledge::{
		KNOWLEDGE_PAGE_CONTRACT_SCHEMA_V1, KNOWLEDGE_PAGE_REBUILD_SCHEMA_V1,
		KNOWLEDGE_PAGE_SOURCE_COVERAGE_SCHEMA_V1, KNOWLEDGE_PAGE_VERSION_DIFF_SCHEMA_V1,
		KNOWLEDGE_PAGE_WATCH_REBUILD_SCHEMA_V1, KnowledgePageKind, KnowledgeSourceKind,
	},
};
use elf_storage::{
	knowledge::{
		self, KnowledgeDocChunkSource, KnowledgeDocSource, KnowledgeEventSource,
		KnowledgeNoteSource, KnowledgePageLintFindingInsert, KnowledgePageSearchRow,
		KnowledgePageSectionInsert, KnowledgePageSourceRefInsert, KnowledgePageUpsert,
		KnowledgeProposalSource, KnowledgeRelationSource, KnowledgeRelationSourcesFetch,
	},
	models::{
		KnowledgePage, KnowledgePageLintFinding, KnowledgePageSection, KnowledgePageSourceRef,
	},
};

const DEFAULT_LIST_LIMIT: i64 = 50;
const MAX_LIST_LIMIT: i64 = 200;
const SEARCH_SNIPPET_CHARS: usize = 280;
const PREVIOUS_VERSION_DIFF_KEY: &str = "previous_version_diff";

/// Request to rebuild one derived knowledge page from explicit source ids.
#[derive(Clone, Debug, Deserialize)]
pub struct KnowledgePageRebuildRequest {
	/// Tenant that owns the page and source records.
	pub tenant_id: String,
	/// Project that owns the page and source records.
	pub project_id: String,
	/// Agent requesting the rebuild.
	pub agent_id: String,
	/// Page kind.
	pub page_kind: KnowledgePageKind,
	/// Stable page key within the tenant/project/kind namespace.
	pub page_key: String,
	/// Optional display title; a deterministic title is generated when omitted.
	pub title: Option<String>,
	#[serde(default)]
	/// Source Library documents to compile into the page.
	pub doc_ids: Vec<Uuid>,
	#[serde(default)]
	/// Source Library document chunks or spans to compile into the page.
	pub doc_chunk_ids: Vec<Uuid>,
	#[serde(default)]
	/// Memory note sources to compile into the page.
	pub note_ids: Vec<Uuid>,
	#[serde(default)]
	/// Durable add_event audit source ids to compile into the page.
	pub event_ids: Vec<Uuid>,
	#[serde(default)]
	/// Graph relation fact ids to compile into the page.
	pub relation_ids: Vec<Uuid>,
	#[serde(default)]
	/// Applied consolidation proposal ids to compile into the page.
	pub proposal_ids: Vec<Uuid>,
	#[serde(default = "empty_object")]
	/// Provider metadata for nondeterministic or future LLM-derived rebuilds.
	pub provider_metadata: Value,
}

/// Response returned after rebuilding a derived knowledge page.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageRebuildResponse {
	/// Rebuilt page with sections, source refs, and lint findings.
	pub page: KnowledgePageResponse,
}

/// Request to get one derived knowledge page.
#[derive(Clone, Debug, Deserialize)]
pub struct KnowledgePageGetRequest {
	/// Tenant that owns the page.
	pub tenant_id: String,
	/// Project that owns the page.
	pub project_id: String,
	/// Page identifier.
	pub page_id: Uuid,
}

/// Request to list derived knowledge pages.
#[derive(Clone, Debug, Deserialize)]
pub struct KnowledgePagesListRequest {
	/// Tenant that owns the pages.
	pub tenant_id: String,
	/// Project that owns the pages.
	pub project_id: String,
	/// Optional page-kind filter.
	pub page_kind: Option<KnowledgePageKind>,
	/// Maximum number of pages to return.
	pub limit: Option<u32>,
}

/// Response returned by derived knowledge page listing.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePagesListResponse {
	/// Returned pages.
	pub pages: Vec<KnowledgePageSummary>,
}

/// Request to lint one derived knowledge page against current source snapshots.
#[derive(Clone, Debug, Deserialize)]
pub struct KnowledgePageLintRequest {
	/// Tenant that owns the page.
	pub tenant_id: String,
	/// Project that owns the page.
	pub project_id: String,
	/// Page identifier.
	pub page_id: Uuid,
}

/// Request to search derived knowledge page sections.
#[derive(Clone, Debug, Deserialize)]
pub struct KnowledgePageSearchRequest {
	/// Tenant that owns the pages.
	pub tenant_id: String,
	/// Project that owns the pages.
	pub project_id: String,
	/// Agent requesting the page search.
	pub agent_id: String,
	/// Read profile controlling source visibility.
	pub read_profile: String,
	/// English-only query for page title, key, heading, or section content.
	pub query: String,
	/// Optional page-kind filter.
	pub page_kind: Option<KnowledgePageKind>,
	/// Maximum number of section snippets to return.
	pub limit: Option<u32>,
}

/// Request to rebuild pages affected by changed authoritative sources.
#[derive(Clone, Debug, Deserialize)]
pub struct KnowledgePageWatchRebuildRequest {
	/// Tenant that owns the pages and changed sources.
	pub tenant_id: String,
	/// Project that owns the pages and changed sources.
	pub project_id: String,
	/// Agent requesting the watch/rebuild operation.
	pub agent_id: String,
	/// Changed source references observed by a watcher or operator.
	pub changed_sources: Vec<KnowledgePageChangedSource>,
	/// Optional page-kind filter for the affected-page lookup.
	pub page_kind: Option<KnowledgePageKind>,
	/// Maximum number of affected pages to rebuild.
	pub limit: Option<u32>,
	#[serde(default = "default_generate_memory_candidates")]
	/// Whether changed knowledge deltas should queue reviewable memory proposals.
	pub generate_memory_candidates: bool,
}

/// Changed authoritative source reference for the watch/rebuild loop.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KnowledgePageChangedSource {
	/// Changed source kind.
	pub source_kind: KnowledgeSourceKind,
	/// Changed source identifier.
	pub source_id: Uuid,
}

/// Response returned after linting one knowledge page.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageLintResponse {
	/// Page identifier.
	pub page_id: Uuid,
	/// Current lint findings.
	pub findings: Vec<KnowledgePageLintFindingResponse>,
}

/// Response returned by derived knowledge page section search.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageSearchResponse {
	/// Matching derived page snippets.
	pub items: Vec<KnowledgePageSearchItem>,
}

/// Response returned after rebuilding pages affected by changed sources.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageWatchRebuildResponse {
	/// Versioned response schema.
	pub schema: String,
	/// Operator-readable aggregate summary.
	pub summary: KnowledgePageWatchRebuildSummary,
	/// Per-page rebuild results.
	pub pages: Vec<KnowledgePageWatchRebuildItem>,
	/// Reviewable memory candidates derived from knowledge deltas.
	pub memory_candidates: Vec<KnowledgeDeltaMemoryCandidate>,
	/// Queued consolidation run, when memory candidates were generated.
	pub proposal_run: Option<KnowledgePageProposalRunSummary>,
	/// One-line operator summary messages.
	pub operator_summary: Vec<String>,
}

/// Aggregate watch/rebuild outcome counters.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageWatchRebuildSummary {
	/// Changed source count after de-duplication.
	pub changed_source_count: usize,
	/// Knowledge pages that cited one of the changed sources.
	pub affected_page_count: usize,
	/// Pages rebuilt with changed derived output.
	pub changed_page_count: usize,
	/// Pages rebuilt with unchanged derived output.
	pub unchanged_page_count: usize,
	/// Pages that had stale lint findings before rebuild.
	pub stale_page_count: usize,
	/// Pages that could not be rebuilt.
	pub blocked_page_count: usize,
	/// Memory candidates generated for review.
	pub memory_candidate_count: usize,
}

/// Per-page changed-source rebuild result.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageWatchRebuildItem {
	/// Knowledge page identifier.
	pub page_id: Uuid,
	/// Page kind.
	pub page_kind: String,
	/// Stable page key.
	pub page_key: String,
	/// Page title.
	pub title: String,
	/// Page rebuild state: changed, unchanged, stale, or blocked.
	pub rebuild_state: String,
	/// Per-section rebuild states.
	pub sections: Vec<KnowledgePageSectionRebuildState>,
	/// Classified rebuild/lint outputs.
	pub outputs: Vec<KnowledgePageRebuildOutput>,
	/// Rebuilt page readback, omitted when blocked.
	pub rebuilt_page: Option<KnowledgePageResponse>,
	/// Blocking error text, when rebuild failed.
	pub blocked_reason: Option<String>,
	/// Previous-version diff metadata, when available.
	pub previous_version_diff: Option<Value>,
	/// Operator-readable page summary.
	pub operator_summary: String,
}

/// Per-section rebuild state for changed-source rebuild output.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageSectionRebuildState {
	/// Stable section key.
	pub section_key: String,
	/// Section heading.
	pub heading: String,
	/// Section state: changed, unchanged, stale, or blocked.
	pub state: String,
	/// Output types attached to the section.
	pub output_types: Vec<String>,
	/// Lint finding types attached to the section before rebuild.
	pub lint_finding_types: Vec<String>,
}

/// Classified output emitted by the watch/rebuild loop.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageRebuildOutput {
	/// Output type, such as stale_section, changed_claim, missing_citation, conflict,
	/// changed_source, or blocked.
	pub output_type: String,
	/// Severity for operator triage.
	pub severity: String,
	/// Associated section key, when section-scoped.
	pub section_key: Option<String>,
	/// Associated source kind, when source-scoped.
	pub source_kind: Option<String>,
	/// Associated source id, when source-scoped.
	pub source_id: Option<Uuid>,
	/// Human-readable output message.
	pub message: String,
	/// Structured reason and evidence details.
	pub details: Value,
}

/// Reviewable memory candidate produced from a knowledge delta.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgeDeltaMemoryCandidate {
	/// Candidate reason, such as changed_claim or conflict.
	pub reason: String,
	/// Knowledge page identifier.
	pub page_id: Uuid,
	/// Section identifier that produced the candidate.
	pub section_id: Uuid,
	/// Stable section key.
	pub section_key: String,
	/// Source refs copied into the reviewable proposal.
	pub source_refs: Vec<ConsolidationInputRef>,
	/// Source snapshot summary for reviewer inspection.
	pub source_snapshot: Value,
	/// Reviewable proposal diff.
	pub diff: ConsolidationProposalDiff,
	/// Proposed memory note payload.
	pub proposed_payload: Value,
}

/// Queued reviewable proposal run produced by changed-source rebuild.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageProposalRunSummary {
	/// Consolidation run identifier.
	pub run_id: Uuid,
	/// Queued worker job identifier.
	pub job_id: Uuid,
	/// Number of memory candidate proposals queued in the run payload.
	pub proposal_count: usize,
	/// Review surface for the queued candidates.
	pub review_surface: String,
}

/// Summary DTO for one derived knowledge page.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageSummary {
	/// Page identifier.
	pub page_id: Uuid,
	/// Tenant that owns the page.
	pub tenant_id: String,
	/// Project that owns the page.
	pub project_id: String,
	/// Page kind.
	pub page_kind: String,
	/// Stable page key.
	pub page_key: String,
	/// Page title.
	pub title: String,
	/// Versioned page contract schema.
	pub contract_schema: String,
	/// Page lifecycle status.
	pub status: String,
	/// Canonical source snapshot hash.
	pub rebuild_source_hash: String,
	/// Canonical page content hash.
	pub content_hash: String,
	/// Source coverage metadata.
	pub source_coverage: Value,
	/// Rebuild metadata.
	pub rebuild_metadata: Value,
	/// Previous-version diff metadata, when present.
	pub previous_version_diff: Option<Value>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
	/// Last rebuild timestamp.
	pub rebuilt_at: OffsetDateTime,
}
impl From<KnowledgePage> for KnowledgePageSummary {
	fn from(page: KnowledgePage) -> Self {
		Self {
			page_id: page.page_id,
			tenant_id: page.tenant_id,
			project_id: page.project_id,
			page_kind: page.page_kind,
			page_key: page.page_key,
			title: page.title,
			contract_schema: page.contract_schema,
			status: page.status,
			rebuild_source_hash: page.rebuild_source_hash,
			content_hash: page.content_hash,
			source_coverage: page.source_coverage,
			previous_version_diff: previous_version_diff_from_metadata(&page.rebuild_metadata),
			rebuild_metadata: page.rebuild_metadata,
			created_at: page.created_at,
			updated_at: page.updated_at,
			rebuilt_at: page.rebuilt_at,
		}
	}
}

/// Full readback DTO for one derived knowledge page.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageResponse {
	/// Page summary.
	pub page: KnowledgePageSummary,
	/// Page sections.
	pub sections: Vec<KnowledgePageSectionResponse>,
	/// Normalized source refs.
	pub source_refs: Vec<KnowledgePageSourceRefResponse>,
	/// Lint findings.
	pub lint_findings: Vec<KnowledgePageLintFindingResponse>,
}

/// Readback DTO for one page section.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageSectionResponse {
	/// Section identifier.
	pub section_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Stable section key.
	pub section_key: String,
	/// Section heading.
	pub heading: String,
	/// Section role.
	pub role: String,
	/// Section content.
	pub content: String,
	/// Display order.
	pub ordinal: i32,
	/// Serialized citation array.
	pub citations: Value,
	/// Reason this section is intentionally unsupported, when present.
	pub unsupported_reason: Option<String>,
	/// Count of section-local citations.
	pub citation_count: usize,
	/// Count of normalized source refs attached to this section.
	pub source_ref_count: usize,
	/// True when the section has both citations and normalized source backlinks.
	pub coverage_complete: bool,
	/// Section-local normalized source backlinks.
	pub source_backlinks: Vec<KnowledgePageSectionSourceBacklink>,
	/// Section content hash.
	pub content_hash: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}
impl From<KnowledgePageSection> for KnowledgePageSectionResponse {
	fn from(section: KnowledgePageSection) -> Self {
		Self {
			section_id: section.section_id,
			page_id: section.page_id,
			section_key: section.section_key,
			heading: section.heading,
			role: section.role,
			content: section.content,
			ordinal: section.ordinal,
			citations: section.citations,
			unsupported_reason: section.unsupported_reason,
			citation_count: 0,
			source_ref_count: 0,
			coverage_complete: false,
			source_backlinks: Vec::new(),
			content_hash: section.content_hash,
			created_at: section.created_at,
			updated_at: section.updated_at,
		}
	}
}

/// Section-local source backlink used by page readback and viewer provenance.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageSectionSourceBacklink {
	/// Source kind.
	pub source_kind: String,
	/// Authoritative source identifier.
	pub source_id: Uuid,
	/// Captured source status.
	pub source_status: Option<String>,
	/// Captured source update timestamp.
	pub source_updated_at: Option<OffsetDateTime>,
	/// Captured source content hash.
	pub source_content_hash: Option<String>,
}
impl From<&KnowledgePageSourceRef> for KnowledgePageSectionSourceBacklink {
	fn from(source_ref: &KnowledgePageSourceRef) -> Self {
		Self {
			source_kind: source_ref.source_kind.clone(),
			source_id: source_ref.source_id,
			source_status: source_ref.source_status.clone(),
			source_updated_at: source_ref.source_updated_at,
			source_content_hash: source_ref.source_content_hash.clone(),
		}
	}
}

/// Readback DTO for one normalized source reference.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageSourceRefResponse {
	/// Source-reference row identifier.
	pub ref_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Citing section, when section-scoped.
	pub section_id: Option<Uuid>,
	/// Source kind.
	pub source_kind: String,
	/// Authoritative source identifier.
	pub source_id: Uuid,
	/// Captured source status.
	pub source_status: Option<String>,
	/// Captured source update timestamp.
	pub source_updated_at: Option<OffsetDateTime>,
	/// Captured source content hash.
	pub source_content_hash: Option<String>,
	/// Captured source snapshot.
	pub source_snapshot: Value,
	/// Citation-local metadata.
	pub citation_metadata: Value,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}
impl From<KnowledgePageSourceRef> for KnowledgePageSourceRefResponse {
	fn from(source_ref: KnowledgePageSourceRef) -> Self {
		Self {
			ref_id: source_ref.ref_id,
			page_id: source_ref.page_id,
			section_id: source_ref.section_id,
			source_kind: source_ref.source_kind,
			source_id: source_ref.source_id,
			source_status: source_ref.source_status,
			source_updated_at: source_ref.source_updated_at,
			source_content_hash: source_ref.source_content_hash,
			source_snapshot: source_ref.source_snapshot,
			citation_metadata: source_ref.citation_metadata,
			created_at: source_ref.created_at,
		}
	}
}

/// Readback DTO for one knowledge page lint finding.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageLintFindingResponse {
	/// Lint finding identifier.
	pub finding_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Associated section, when available.
	pub section_id: Option<Uuid>,
	/// Finding type.
	pub finding_type: String,
	/// Finding severity.
	pub severity: String,
	/// Source kind associated with the finding, when available.
	pub source_kind: Option<String>,
	/// Source identifier associated with the finding, when available.
	pub source_id: Option<Uuid>,
	/// Human-readable finding message.
	pub message: String,
	/// Structured finding details.
	pub details: Value,
	/// Operator guidance for repair or rebuild.
	pub repair_guidance: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}
impl From<KnowledgePageLintFinding> for KnowledgePageLintFindingResponse {
	fn from(finding: KnowledgePageLintFinding) -> Self {
		let repair_guidance =
			repair_guidance_for_finding_type(finding.finding_type.as_str()).to_string();

		Self {
			finding_id: finding.finding_id,
			page_id: finding.page_id,
			section_id: finding.section_id,
			finding_type: finding.finding_type,
			severity: finding.severity,
			source_kind: finding.source_kind,
			source_id: finding.source_id,
			message: finding.message,
			repair_guidance,
			details: finding.details,
			created_at: finding.created_at,
		}
	}
}

/// Search result for one derived knowledge page section.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageSearchItem {
	/// Result type discriminator for clients that mix pages with notes.
	pub result_kind: String,
	/// Derived page identifier.
	pub page_id: Uuid,
	/// Page kind.
	pub page_kind: String,
	/// Stable page key.
	pub page_key: String,
	/// Page title.
	pub title: String,
	/// Page lifecycle status.
	pub status: String,
	/// Section identifier.
	pub section_id: Uuid,
	/// Stable section key.
	pub section_key: String,
	/// Section heading.
	pub heading: String,
	/// Section role.
	pub role: String,
	/// Bounded matching section snippet.
	pub snippet: String,
	/// Section citations for visible provenance.
	pub citations: Value,
	/// Count of section-local citations.
	pub citation_count: usize,
	/// Count of normalized source refs attached to this section.
	pub source_ref_count: usize,
	/// Section-local source refs for backlink readback.
	pub source_refs: Vec<KnowledgePageSourceRefResponse>,
	/// Page-level source coverage metadata.
	pub source_coverage: Value,
	/// Page-level rebuild metadata.
	pub rebuild_metadata: Value,
	/// Previous-version diff metadata, when present.
	pub previous_version_diff: Option<Value>,
	/// Lint summary for distinguishing clean, stale, and unsupported pages.
	pub lint_summary: KnowledgePageLintSummary,
	/// Trust state discriminator for viewer/search clients.
	pub trust_state: String,
	/// Explicit notice that the result is derived, not authoritative source truth.
	pub derived_notice: String,
	/// Repair or rebuild guidance when lint or coverage indicates risk.
	pub repair_guidance: Option<String>,
	/// Page update timestamp.
	pub updated_at: OffsetDateTime,
	/// Page rebuild timestamp.
	pub rebuilt_at: OffsetDateTime,
}

/// Aggregate lint counts for page search results.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageLintSummary {
	/// Error finding count.
	pub error_count: i64,
	/// Warning finding count.
	pub warning_count: i64,
	/// Info finding count.
	pub info_count: i64,
	/// True when at least one error finding exists.
	pub has_errors: bool,
	/// True when at least one warning finding exists.
	pub has_warnings: bool,
}

#[derive(Clone, Debug)]
struct SourceSnapshot {
	kind: KnowledgeSourceKind,
	id: Uuid,
	status: Option<String>,
	updated_at: Option<OffsetDateTime>,
	content_hash: Option<String>,
	snapshot: Value,
	citation_metadata: Value,
	line: String,
}

#[derive(Clone, Debug)]
struct DraftSection {
	section_id: Uuid,
	section_key: String,
	heading: String,
	role: String,
	content: String,
	ordinal: i32,
	source_indexes: Vec<usize>,
	unsupported_reason: Option<String>,
	content_hash: String,
	citations: Value,
}

#[derive(Clone, Debug)]
struct LintDraft {
	section_id: Option<Uuid>,
	finding_type: String,
	severity: String,
	source_kind: Option<KnowledgeSourceKind>,
	source_id: Option<Uuid>,
	message: String,
	details: Value,
}

#[derive(Clone, Debug)]
struct SourceIds {
	doc_ids: Vec<Uuid>,
	doc_chunk_ids: Vec<Uuid>,
	note_ids: Vec<Uuid>,
	event_ids: Vec<Uuid>,
	relation_ids: Vec<Uuid>,
	proposal_ids: Vec<Uuid>,
}
impl SourceIds {
	fn from_request(req: &KnowledgePageRebuildRequest) -> Result<Self> {
		let ids = Self {
			doc_ids: sorted_unique(&req.doc_ids),
			doc_chunk_ids: sorted_unique(&req.doc_chunk_ids),
			note_ids: sorted_unique(&req.note_ids),
			event_ids: sorted_unique(&req.event_ids),
			relation_ids: sorted_unique(&req.relation_ids),
			proposal_ids: sorted_unique(&req.proposal_ids),
		};

		ids.validate_non_empty()?;

		Ok(ids)
	}

	fn from_source_refs(source_refs: &[KnowledgePageSourceRef]) -> Result<Self> {
		let mut doc_ids = Vec::new();
		let mut doc_chunk_ids = Vec::new();
		let mut note_ids = Vec::new();
		let mut event_ids = Vec::new();
		let mut relation_ids = Vec::new();
		let mut proposal_ids = Vec::new();

		for source_ref in source_refs {
			match KnowledgeSourceKind::parse(source_ref.source_kind.as_str()) {
				Some(KnowledgeSourceKind::Doc) => doc_ids.push(source_ref.source_id),
				Some(KnowledgeSourceKind::DocChunk) => doc_chunk_ids.push(source_ref.source_id),
				Some(KnowledgeSourceKind::Note) => note_ids.push(source_ref.source_id),
				Some(KnowledgeSourceKind::Event) => event_ids.push(source_ref.source_id),
				Some(KnowledgeSourceKind::Relation) => relation_ids.push(source_ref.source_id),
				Some(KnowledgeSourceKind::Proposal) => proposal_ids.push(source_ref.source_id),
				None => {
					return Err(Error::InvalidRequest {
						message: "stored knowledge page source kind is invalid".to_string(),
					});
				},
			}
		}

		Ok(Self {
			doc_ids: sorted_unique(&doc_ids),
			doc_chunk_ids: sorted_unique(&doc_chunk_ids),
			note_ids: sorted_unique(&note_ids),
			event_ids: sorted_unique(&event_ids),
			relation_ids: sorted_unique(&relation_ids),
			proposal_ids: sorted_unique(&proposal_ids),
		})
	}

	fn validate_non_empty(&self) -> Result<()> {
		if self.doc_ids.is_empty()
			&& self.doc_chunk_ids.is_empty()
			&& self.note_ids.is_empty()
			&& self.event_ids.is_empty()
			&& self.relation_ids.is_empty()
			&& self.proposal_ids.is_empty()
		{
			return Err(Error::InvalidRequest {
				message: "at least one source id is required for a knowledge page rebuild"
					.to_string(),
			});
		}

		Ok(())
	}

	fn require_counts(
		&self,
		docs: usize,
		doc_chunks: usize,
		notes: usize,
		events: usize,
		relations: usize,
		proposals: usize,
	) -> Result<()> {
		if docs != self.doc_ids.len()
			|| doc_chunks != self.doc_chunk_ids.len()
			|| notes != self.note_ids.len()
			|| events != self.event_ids.len()
			|| relations != self.relation_ids.len()
			|| proposals != self.proposal_ids.len()
		{
			return Err(Error::InvalidRequest {
				message:
					"all requested knowledge page sources must exist, source rows must be active and readable, and proposals must be applied"
						.to_string(),
			});
		}

		Ok(())
	}
}

struct WatchRebuildOutcome {
	item: KnowledgePageWatchRebuildItem,
	candidates: Vec<KnowledgeDeltaMemoryCandidate>,
}
impl ElfService {
	/// Rebuilds and persists one derived knowledge page from explicit source ids.
	pub async fn knowledge_page_rebuild(
		&self,
		req: KnowledgePageRebuildRequest,
	) -> Result<KnowledgePageRebuildResponse> {
		validate_context(req.tenant_id.as_str(), req.project_id.as_str(), req.agent_id.as_str())?;
		validate_non_empty("page_key", req.page_key.as_str())?;
		validate_object("provider_metadata", &req.provider_metadata)?;

		let ids = SourceIds::from_request(&req)?;
		let title =
			req.title.clone().unwrap_or_else(|| generated_title(req.page_kind, &req.page_key));
		let previous_page = knowledge::get_knowledge_page_by_key(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.page_kind.as_str(),
			req.page_key.as_str(),
		)
		.await?;
		let previous_sections = match &previous_page {
			Some(page) =>
				knowledge::list_knowledge_page_sections(&self.db.pool, page.page_id).await?,
			None => Vec::new(),
		};
		let sources = self.resolve_sources(&req, &ids).await?;
		let now = OffsetDateTime::now_utc();
		let source_snapshot = source_snapshot_value(&sources);
		let source_hash = hash_json(&source_snapshot)?;
		let mut sections = build_sections(&sources)?;
		let lint = lint_unsupported_sections(&sections);

		for section in &mut sections {
			section.citations = citations_value(section, &sources);
			section.content_hash = hash_json(&section_hash_payload(section))?;
		}

		let source_coverage =
			source_coverage_value(req.page_kind, &req.page_key, &sections, &sources);
		let base_rebuild_metadata = rebuild_metadata(&source_hash, &req.provider_metadata, &req);
		let content_hash =
			page_content_hash(&title, &sections, &source_coverage, &base_rebuild_metadata)?;
		let previous_version_diff = previous_version_diff_value(
			previous_page.as_ref(),
			&previous_sections,
			title.as_str(),
			source_hash.as_str(),
			content_hash.as_str(),
			&sections,
		);
		let version_identity = version_identity_value(
			req.page_kind,
			req.page_key.as_str(),
			source_hash.as_str(),
			content_hash.as_str(),
			&sections,
		);
		let rebuild_metadata = rebuild_metadata_with_previous_version_diff(
			base_rebuild_metadata,
			previous_version_diff,
			version_identity,
		);
		let page_id = Uuid::new_v4();
		let mut tx = self.db.pool.begin().await?;
		let page = knowledge::upsert_knowledge_page(
			&mut *tx,
			KnowledgePageUpsert {
				page_id,
				tenant_id: req.tenant_id.as_str(),
				project_id: req.project_id.as_str(),
				page_kind: req.page_kind.as_str(),
				page_key: req.page_key.as_str(),
				title: title.as_str(),
				contract_schema: KNOWLEDGE_PAGE_CONTRACT_SCHEMA_V1,
				status: "active",
				rebuild_source_hash: source_hash.as_str(),
				content_hash: content_hash.as_str(),
				source_coverage: &source_coverage,
				source_snapshot: &source_snapshot,
				rebuild_metadata: &rebuild_metadata,
				now,
			},
		)
		.await?;

		replace_page_children(&mut tx, page.page_id, &sections, &sources, &lint, now).await?;

		tx.commit().await?;

		Ok(KnowledgePageRebuildResponse { page: self.knowledge_page_response(page).await? })
	}

	/// Rebuilds pages affected by changed source refs and queues reviewable candidates.
	pub async fn knowledge_pages_watch_rebuild(
		&self,
		req: KnowledgePageWatchRebuildRequest,
	) -> Result<KnowledgePageWatchRebuildResponse> {
		validate_context(req.tenant_id.as_str(), req.project_id.as_str(), req.agent_id.as_str())?;

		let changed_sources = normalized_changed_sources(&req.changed_sources)?;
		let (source_kinds, source_ids) = changed_source_arrays(&changed_sources);
		let page_kind = req.page_kind.map(KnowledgePageKind::as_str);
		let pages = knowledge::list_knowledge_pages_for_sources(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			page_kind,
			&source_kinds,
			&source_ids,
			bounded_limit(req.limit),
		)
		.await?;
		let mut items = Vec::new();
		let mut candidates = Vec::new();

		for page in pages {
			let outcome =
				self.watch_rebuild_page(req.agent_id.as_str(), page, &changed_sources).await?;

			candidates.extend(outcome.candidates);
			items.push(outcome.item);
		}

		let proposal_run = if req.generate_memory_candidates && !candidates.is_empty() {
			Some(self.queue_knowledge_delta_candidates(&req, &changed_sources, &candidates).await?)
		} else {
			None
		};
		let summary = watch_rebuild_summary(changed_sources.len(), &items, candidates.len());
		let operator_summary = watch_operator_summary(&summary, proposal_run.as_ref());

		Ok(KnowledgePageWatchRebuildResponse {
			schema: KNOWLEDGE_PAGE_WATCH_REBUILD_SCHEMA_V1.to_string(),
			summary,
			pages: items,
			memory_candidates: candidates,
			proposal_run,
			operator_summary,
		})
	}

	/// Gets one derived knowledge page with sections, source refs, and lint findings.
	pub async fn knowledge_page_get(
		&self,
		req: KnowledgePageGetRequest,
	) -> Result<KnowledgePageResponse> {
		let page = knowledge::get_knowledge_page(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.page_id,
		)
		.await?
		.ok_or_else(|| Error::NotFound { message: "knowledge page not found".to_string() })?;

		self.knowledge_page_response(page).await
	}

	/// Lists derived knowledge pages.
	pub async fn knowledge_pages_list(
		&self,
		req: KnowledgePagesListRequest,
	) -> Result<KnowledgePagesListResponse> {
		let page_kind = req.page_kind.map(KnowledgePageKind::as_str);
		let pages = knowledge::list_knowledge_pages(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			page_kind,
			bounded_limit(req.limit),
		)
		.await?
		.into_iter()
		.map(KnowledgePageSummary::from)
		.collect();

		Ok(KnowledgePagesListResponse { pages })
	}

	/// Searches derived knowledge page sections and returns provenance-rich snippets.
	pub async fn knowledge_pages_search(
		&self,
		req: KnowledgePageSearchRequest,
	) -> Result<KnowledgePageSearchResponse> {
		validate_non_empty("tenant_id", req.tenant_id.as_str())?;
		validate_non_empty("project_id", req.project_id.as_str())?;
		validate_non_empty("agent_id", req.agent_id.as_str())?;
		validate_non_empty("read_profile", req.read_profile.as_str())?;
		validate_non_empty("query", req.query.as_str())?;

		if !english_gate::is_english_natural_language(req.query.as_str()) {
			return Err(Error::NonEnglishInput { field: "$.query".to_string() });
		}

		let allowed_scopes =
			search::resolve_read_profile_scopes(&self.cfg, req.read_profile.as_str())?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.agent_id.as_str(),
			org_shared_allowed,
		)
		.await?;
		let query = req.query.trim().to_ascii_lowercase();
		let query_pattern = format!("%{query}%");
		let page_kind = req.page_kind.map(KnowledgePageKind::as_str);
		let rows = knowledge::search_knowledge_page_sections(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			page_kind,
			query_pattern.as_str(),
			bounded_limit(req.limit),
		)
		.await?;
		let page_ids = sorted_unique(&rows.iter().map(|row| row.page_id).collect::<Vec<_>>());
		let source_refs =
			knowledge::list_knowledge_page_source_refs_for_pages(&self.db.pool, &page_ids).await?;
		let current_source_keys = self
			.resolve_current_recallable_source_keys(
				req.tenant_id.as_str(),
				req.project_id.as_str(),
				req.agent_id.as_str(),
				&allowed_scopes,
				&shared_grants,
				&source_refs,
			)
			.await?;
		let source_refs_by_section = source_refs_by_section(&source_refs);
		let items = rows
			.into_iter()
			.filter_map(|row| {
				let refs = cloned_source_refs(source_refs_by_section.get(&row.section_id));

				recallable_source_refs(refs.as_slice(), &current_source_keys)
					.then(|| knowledge_page_search_item(row, refs, req.query.as_str()))
			})
			.collect();

		Ok(KnowledgePageSearchResponse { items })
	}

	/// Lints a derived knowledge page against current source snapshots.
	pub async fn knowledge_page_lint(
		&self,
		req: KnowledgePageLintRequest,
	) -> Result<KnowledgePageLintResponse> {
		let page = knowledge::get_knowledge_page(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.page_id,
		)
		.await?
		.ok_or_else(|| Error::NotFound { message: "knowledge page not found".to_string() })?;
		let source_refs =
			knowledge::list_knowledge_page_source_refs(&self.db.pool, page.page_id).await?;
		let sections = knowledge::list_knowledge_page_sections(&self.db.pool, page.page_id).await?;
		let mut findings = self.lint_source_refs(&page, &source_refs).await?;

		findings.extend(lint_page_sections(&page, &sections, &source_refs));

		let now = OffsetDateTime::now_utc();
		let mut tx = self.db.pool.begin().await?;

		knowledge::delete_knowledge_page_lint_findings(&mut *tx, page.page_id).await?;

		for finding in &findings {
			insert_lint_finding(&mut tx, page.page_id, finding, now).await?;
		}

		tx.commit().await?;

		let persisted = knowledge::list_knowledge_page_lint_findings(&self.db.pool, page.page_id)
			.await?
			.into_iter()
			.map(KnowledgePageLintFindingResponse::from)
			.collect();

		Ok(KnowledgePageLintResponse { page_id: page.page_id, findings: persisted })
	}

	async fn knowledge_page_response(&self, page: KnowledgePage) -> Result<KnowledgePageResponse> {
		let page_id = page.page_id;
		let section_rows = knowledge::list_knowledge_page_sections(&self.db.pool, page_id).await?;
		let source_ref_rows =
			knowledge::list_knowledge_page_source_refs(&self.db.pool, page_id).await?;
		let source_refs_by_section = source_refs_by_section(&source_ref_rows);
		let sections = section_rows
			.into_iter()
			.map(|section| {
				let refs = cloned_source_refs(source_refs_by_section.get(&section.section_id));

				section_response(section, refs)
			})
			.collect();
		let source_refs =
			source_ref_rows.into_iter().map(KnowledgePageSourceRefResponse::from).collect();
		let lint_findings = knowledge::list_knowledge_page_lint_findings(&self.db.pool, page_id)
			.await?
			.into_iter()
			.map(KnowledgePageLintFindingResponse::from)
			.collect();

		Ok(KnowledgePageResponse {
			page: KnowledgePageSummary::from(page),
			sections,
			source_refs,
			lint_findings,
		})
	}

	async fn resolve_sources(
		&self,
		req: &KnowledgePageRebuildRequest,
		ids: &SourceIds,
	) -> Result<Vec<SourceSnapshot>> {
		let allowed_scopes = self.cfg.scopes.allowed.as_slice();
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.agent_id.as_str(),
			org_shared_allowed,
		)
		.await?;
		let (docs, doc_chunks, notes, events, relations, proposals) = self
			.resolve_existing_source_rows(
				req.tenant_id.as_str(),
				req.project_id.as_str(),
				Some(req.agent_id.as_str()),
				allowed_scopes,
				&shared_grants,
				ids,
			)
			.await?;

		ids.require_counts(
			docs.len(),
			doc_chunks.len(),
			notes.len(),
			events.len(),
			relations.len(),
			proposals.len(),
		)?;

		Ok(source_snapshots(docs, doc_chunks, notes, events, relations, proposals))
	}

	async fn resolve_existing_source_rows(
		&self,
		tenant_id: &str,
		project_id: &str,
		agent_id: Option<&str>,
		allowed_scopes: &[String],
		shared_grants: &HashSet<access::SharedSpaceGrantKey>,
		ids: &SourceIds,
	) -> Result<(
		Vec<KnowledgeDocSource>,
		Vec<KnowledgeDocChunkSource>,
		Vec<KnowledgeNoteSource>,
		Vec<KnowledgeEventSource>,
		Vec<KnowledgeRelationSource>,
		Vec<KnowledgeProposalSource>,
	)> {
		let docs = knowledge::fetch_knowledge_doc_sources(
			&self.db.pool,
			tenant_id,
			project_id,
			agent_id,
			allowed_scopes,
			&ids.doc_ids,
		)
		.await?;
		let docs = docs
			.into_iter()
			.filter(|source| {
				source_row_read_allowed(
					source.agent_id.as_str(),
					source.scope.as_str(),
					agent_id,
					allowed_scopes,
					shared_grants,
				)
			})
			.collect();
		let doc_chunks = knowledge::fetch_knowledge_doc_chunk_sources(
			&self.db.pool,
			tenant_id,
			project_id,
			agent_id,
			allowed_scopes,
			&ids.doc_chunk_ids,
		)
		.await?;
		let doc_chunks = doc_chunks
			.into_iter()
			.filter(|source| {
				source_row_read_allowed(
					source.agent_id.as_str(),
					source.scope.as_str(),
					agent_id,
					allowed_scopes,
					shared_grants,
				)
			})
			.collect();
		let notes = knowledge::fetch_knowledge_note_sources(
			&self.db.pool,
			tenant_id,
			project_id,
			agent_id,
			allowed_scopes,
			&ids.note_ids,
		)
		.await?;
		let notes = notes
			.into_iter()
			.filter(|source| {
				source_row_read_allowed(
					source.agent_id.as_str(),
					source.scope.as_str(),
					agent_id,
					allowed_scopes,
					shared_grants,
				)
			})
			.collect();
		let events = knowledge::fetch_knowledge_event_sources(
			&self.db.pool,
			tenant_id,
			project_id,
			agent_id,
			allowed_scopes,
			&ids.event_ids,
		)
		.await?;
		let events = events
			.into_iter()
			.filter(|source| {
				source_row_read_allowed(
					source.agent_id.as_str(),
					source.scope.as_str(),
					agent_id,
					allowed_scopes,
					shared_grants,
				)
			})
			.collect();
		let shared_scope_keys = access::shared_scope_key_strings(shared_grants);
		let private_allowed = allowed_scopes.iter().any(|scope| scope == "agent_private");
		let relations = knowledge::fetch_knowledge_relation_sources(
			&self.db.pool,
			KnowledgeRelationSourcesFetch {
				tenant_id,
				project_id,
				agent_id,
				allowed_scopes,
				shared_scope_keys: shared_scope_keys.as_slice(),
				private_allowed,
				fact_ids: &ids.relation_ids,
			},
		)
		.await?;
		let proposals = knowledge::fetch_knowledge_proposal_sources(
			&self.db.pool,
			tenant_id,
			project_id,
			&ids.proposal_ids,
		)
		.await?;

		Ok((docs, doc_chunks, notes, events, relations, proposals))
	}

	async fn lint_source_refs(
		&self,
		page: &KnowledgePage,
		source_refs: &[KnowledgePageSourceRef],
	) -> Result<Vec<LintDraft>> {
		let ids = SourceIds::from_source_refs(source_refs)?;
		let current = self.resolve_current_source_map(page, &ids).await?;
		let mut findings = Vec::new();

		for source_ref in source_refs {
			let key = current_key(source_ref.source_kind.as_str(), source_ref.source_id);
			let Some(snapshot) = current.get(&key) else {
				findings.push(missing_source_finding(source_ref));

				continue;
			};

			if source_changed(source_ref, snapshot) {
				findings.push(stale_source_finding(source_ref, snapshot));
			}
		}

		Ok(findings)
	}

	async fn resolve_current_source_map(
		&self,
		page: &KnowledgePage,
		ids: &SourceIds,
	) -> Result<BTreeMap<String, SourceSnapshot>> {
		let _page_kind = KnowledgePageKind::parse(page.page_kind.as_str()).ok_or_else(|| {
			Error::InvalidRequest { message: "stored knowledge page kind is invalid".to_string() }
		})?;
		let (docs, doc_chunks, notes, events, relations, proposals) = self
			.resolve_existing_source_rows(
				page.tenant_id.as_str(),
				page.project_id.as_str(),
				None,
				self.cfg.scopes.allowed.as_slice(),
				&HashSet::new(),
				ids,
			)
			.await?;
		let mut sources = source_snapshots(docs, doc_chunks, notes, events, relations, proposals);

		Ok(sources.drain(..).map(|source| (source_key(&source), source)).collect())
	}

	async fn resolve_current_recallable_source_keys(
		&self,
		tenant_id: &str,
		project_id: &str,
		agent_id: &str,
		allowed_scopes: &[String],
		shared_grants: &HashSet<access::SharedSpaceGrantKey>,
		source_refs: &[KnowledgePageSourceRef],
	) -> Result<BTreeSet<String>> {
		let ids = SourceIds::from_source_refs(source_refs)?;
		let (docs, doc_chunks, notes, events, relations, proposals) = self
			.resolve_existing_source_rows(
				tenant_id,
				project_id,
				Some(agent_id),
				allowed_scopes,
				shared_grants,
				&ids,
			)
			.await?;

		Ok(source_snapshots(docs, doc_chunks, notes, events, relations, proposals)
			.into_iter()
			.map(|source| source_key(&source))
			.collect())
	}

	async fn watch_rebuild_page(
		&self,
		agent_id: &str,
		page: KnowledgePage,
		changed_sources: &[KnowledgePageChangedSource],
	) -> Result<WatchRebuildOutcome> {
		let source_refs =
			knowledge::list_knowledge_page_source_refs(&self.db.pool, page.page_id).await?;
		let sections = knowledge::list_knowledge_page_sections(&self.db.pool, page.page_id).await?;
		let before_lint = self.watch_rebuild_lint(&page, &sections, &source_refs).await?;
		let request = rebuild_request_from_page(agent_id, &page, &source_refs);
		let rebuild = match request {
			Ok(request) => self.knowledge_page_rebuild(request).await,
			Err(err) => Err(err),
		};

		match rebuild {
			Ok(response) => Ok(successful_watch_rebuild(
				sections,
				source_refs,
				before_lint,
				response.page,
				changed_sources,
			)),
			Err(err) => Ok(blocked_watch_rebuild(page, sections, before_lint, err)),
		}
	}

	async fn watch_rebuild_lint(
		&self,
		page: &KnowledgePage,
		sections: &[KnowledgePageSection],
		source_refs: &[KnowledgePageSourceRef],
	) -> Result<Vec<LintDraft>> {
		let mut lint = self.lint_source_refs(page, source_refs).await?;

		lint.extend(lint_page_sections(page, sections, source_refs));

		Ok(lint)
	}

	async fn queue_knowledge_delta_candidates(
		&self,
		req: &KnowledgePageWatchRebuildRequest,
		changed_sources: &[KnowledgePageChangedSource],
		candidates: &[KnowledgeDeltaMemoryCandidate],
	) -> Result<KnowledgePageProposalRunSummary> {
		let source_refs = candidate_run_input_refs(candidates);
		let source_snapshot = knowledge_delta_source_snapshot(changed_sources, candidates);
		let lineage = ConsolidationLineage {
			source_refs: source_refs.clone(),
			parent_run_id: None,
			parent_proposal_ids: Vec::new(),
		};
		let proposals = candidates.iter().map(candidate_proposal_input).collect::<Vec<_>>();
		let created = self
			.consolidation_run_create(ConsolidationRunCreateRequest {
				tenant_id: req.tenant_id.clone(),
				project_id: req.project_id.clone(),
				agent_id: req.agent_id.clone(),
				job_kind: "manual".to_string(),
				input_refs: source_refs,
				source_snapshot,
				lineage,
				proposals,
			})
			.await?;

		Ok(proposal_run_summary(created, candidates.len()))
	}
}

fn normalized_changed_sources(
	changed_sources: &[KnowledgePageChangedSource],
) -> Result<Vec<KnowledgePageChangedSource>> {
	if changed_sources.is_empty() {
		return Err(Error::InvalidRequest {
			message: "changed_sources must not be empty.".to_string(),
		});
	}

	let mut seen = BTreeSet::new();
	let mut out = Vec::new();

	for source in changed_sources {
		if seen.insert((source.source_kind.as_str().to_string(), source.source_id)) {
			out.push(source.clone());
		}
	}

	Ok(out)
}

fn changed_source_arrays(
	changed_sources: &[KnowledgePageChangedSource],
) -> (Vec<String>, Vec<Uuid>) {
	changed_sources
		.iter()
		.map(|source| (source.source_kind.as_str().to_string(), source.source_id))
		.unzip()
}

fn rebuild_request_from_page(
	agent_id: &str,
	page: &KnowledgePage,
	source_refs: &[KnowledgePageSourceRef],
) -> Result<KnowledgePageRebuildRequest> {
	let ids = SourceIds::from_source_refs(source_refs)?;
	let page_kind = KnowledgePageKind::parse(page.page_kind.as_str()).ok_or_else(|| {
		Error::InvalidRequest { message: "stored knowledge page kind is invalid".to_string() }
	})?;
	let provider_metadata = page
		.rebuild_metadata
		.get("provider_metadata")
		.filter(|metadata| matches!(metadata, Value::Object(_)))
		.cloned()
		.unwrap_or_else(empty_object);

	Ok(KnowledgePageRebuildRequest {
		tenant_id: page.tenant_id.clone(),
		project_id: page.project_id.clone(),
		agent_id: agent_id.to_string(),
		page_kind,
		page_key: page.page_key.clone(),
		title: Some(page.title.clone()),
		doc_ids: ids.doc_ids,
		doc_chunk_ids: ids.doc_chunk_ids,
		note_ids: ids.note_ids,
		event_ids: ids.event_ids,
		relation_ids: ids.relation_ids,
		proposal_ids: ids.proposal_ids,
		provider_metadata,
	})
}

fn successful_watch_rebuild(
	before_sections: Vec<KnowledgePageSection>,
	before_source_refs: Vec<KnowledgePageSourceRef>,
	before_lint: Vec<LintDraft>,
	rebuilt_page: KnowledgePageResponse,
	changed_sources: &[KnowledgePageChangedSource],
) -> WatchRebuildOutcome {
	let previous_version_diff = rebuilt_page.page.previous_version_diff.clone();
	let outputs = rebuild_outputs(
		&before_sections,
		&before_source_refs,
		&before_lint,
		previous_version_diff.as_ref(),
		changed_sources,
	);
	let sections = successful_section_states(&before_sections, &rebuilt_page.sections, &outputs);
	let rebuild_state = successful_rebuild_state(previous_version_diff.as_ref(), &outputs);
	let candidates = memory_candidates_for_page(&rebuilt_page, &outputs);
	let operator_summary = page_operator_summary(
		rebuilt_page.page.page_key.as_str(),
		rebuild_state.as_str(),
		outputs.len(),
		candidates.len(),
	);
	let item = KnowledgePageWatchRebuildItem {
		page_id: rebuilt_page.page.page_id,
		page_kind: rebuilt_page.page.page_kind.clone(),
		page_key: rebuilt_page.page.page_key.clone(),
		title: rebuilt_page.page.title.clone(),
		rebuild_state,
		sections,
		outputs,
		rebuilt_page: Some(rebuilt_page),
		blocked_reason: None,
		previous_version_diff,
		operator_summary,
	};

	WatchRebuildOutcome { item, candidates }
}

fn blocked_watch_rebuild(
	page: KnowledgePage,
	sections: Vec<KnowledgePageSection>,
	before_lint: Vec<LintDraft>,
	err: Error,
) -> WatchRebuildOutcome {
	let outputs = blocked_outputs(&sections, &before_lint, err.to_string().as_str());
	let section_states = blocked_section_states(&sections, &outputs);
	let operator_summary =
		page_operator_summary(page.page_key.as_str(), "blocked", outputs.len(), 0);
	let item = KnowledgePageWatchRebuildItem {
		page_id: page.page_id,
		page_kind: page.page_kind,
		page_key: page.page_key,
		title: page.title,
		rebuild_state: "blocked".to_string(),
		sections: section_states,
		outputs,
		rebuilt_page: None,
		blocked_reason: Some(err.to_string()),
		previous_version_diff: previous_version_diff_from_metadata(&page.rebuild_metadata),
		operator_summary,
	};

	WatchRebuildOutcome { item, candidates: Vec::new() }
}

fn rebuild_outputs(
	sections: &[KnowledgePageSection],
	source_refs: &[KnowledgePageSourceRef],
	lint: &[LintDraft],
	diff: Option<&Value>,
	changed_sources: &[KnowledgePageChangedSource],
) -> Vec<KnowledgePageRebuildOutput> {
	let section_index = section_lookup(sections);
	let changed_keys = diff_section_keys(diff, "changed_section_keys");
	let mut outputs = lint_outputs(lint, &section_index);

	outputs.extend(changed_claim_outputs(sections, &changed_keys));
	outputs.extend(conflict_outputs(&outputs));
	outputs.extend(changed_source_outputs(source_refs, changed_sources));

	outputs
}

fn blocked_outputs(
	sections: &[KnowledgePageSection],
	lint: &[LintDraft],
	blocked_reason: &str,
) -> Vec<KnowledgePageRebuildOutput> {
	let section_index = section_lookup(sections);
	let mut outputs = lint_outputs(lint, &section_index);

	outputs.push(KnowledgePageRebuildOutput {
		output_type: "blocked".to_string(),
		severity: "error".to_string(),
		section_key: None,
		source_kind: None,
		source_id: None,
		message: "Knowledge page could not be rebuilt from its stored source refs.".to_string(),
		details: serde_json::json!({ "blocked_reason": blocked_reason }),
	});

	outputs
}

fn lint_outputs(
	lint: &[LintDraft],
	section_index: &BTreeMap<Uuid, (String, String)>,
) -> Vec<KnowledgePageRebuildOutput> {
	lint.iter().filter_map(|finding| lint_output(finding, section_index)).collect()
}

fn lint_output(
	finding: &LintDraft,
	section_index: &BTreeMap<Uuid, (String, String)>,
) -> Option<KnowledgePageRebuildOutput> {
	let output_type = match finding.finding_type.as_str() {
		"stale_source_ref" => "stale_section",
		"missing_citation" | "missing_source_ref" => "missing_citation",
		_ => return None,
	};
	let (section_key, heading) = finding
		.section_id
		.and_then(|section_id| section_index.get(&section_id))
		.cloned()
		.unwrap_or_else(|| ("page".to_string(), "Page".to_string()));

	Some(KnowledgePageRebuildOutput {
		output_type: output_type.to_string(),
		severity: finding.severity.clone(),
		section_key: Some(section_key.clone()),
		source_kind: finding.source_kind.map(KnowledgeSourceKind::as_str).map(ToString::to_string),
		source_id: finding.source_id,
		message: lint_output_message(output_type, heading.as_str()),
		details: serde_json::json!({
			"finding_type": finding.finding_type,
			"section_key": section_key,
			"lint_details": finding.details,
		}),
	})
}

fn changed_claim_outputs(
	sections: &[KnowledgePageSection],
	changed_keys: &BTreeSet<String>,
) -> Vec<KnowledgePageRebuildOutput> {
	sections
		.iter()
		.filter(|section| changed_keys.contains(section.section_key.as_str()))
		.map(|section| KnowledgePageRebuildOutput {
			output_type: "changed_claim".to_string(),
			severity: "info".to_string(),
			section_key: Some(section.section_key.clone()),
			source_kind: None,
			source_id: None,
			message: format!(
				"Knowledge page section '{}' changed after rebuilding from current sources.",
				section.heading
			),
			details: serde_json::json!({
				"section_key": section.section_key,
				"section_hash": section.content_hash,
			}),
		})
		.collect()
}

fn changed_source_outputs(
	source_refs: &[KnowledgePageSourceRef],
	changed_sources: &[KnowledgePageChangedSource],
) -> Vec<KnowledgePageRebuildOutput> {
	let changed = changed_source_set(changed_sources);

	source_refs
		.iter()
		.filter(|source_ref| {
			changed.contains(&(source_ref.source_kind.clone(), source_ref.source_id))
		})
		.map(|source_ref| KnowledgePageRebuildOutput {
			output_type: "changed_source".to_string(),
			severity: "info".to_string(),
			section_key: None,
			source_kind: Some(source_ref.source_kind.clone()),
			source_id: Some(source_ref.source_id),
			message: "Changed source is attached to this knowledge page.".to_string(),
			details: serde_json::json!({
				"source_kind": source_ref.source_kind,
				"source_id": source_ref.source_id,
				"section_id": source_ref.section_id,
			}),
		})
		.collect()
}

fn conflict_outputs(outputs: &[KnowledgePageRebuildOutput]) -> Vec<KnowledgePageRebuildOutput> {
	let stale = output_section_keys(outputs, "stale_section");
	let changed = output_section_keys(outputs, "changed_claim");

	stale
		.intersection(&changed)
		.map(|section_key| {
			KnowledgePageRebuildOutput {
			output_type: "conflict".to_string(),
			severity: "warning".to_string(),
			section_key: Some(section_key.clone()),
			source_kind: None,
			source_id: None,
			message:
				"Stored derived section was stale and changed after rebuilding from current sources."
					.to_string(),
			details: serde_json::json!({
				"section_key": section_key,
				"reason": "stale_snapshot_changed_claim",
			}),
		}
		})
		.collect()
}

fn successful_section_states(
	before_sections: &[KnowledgePageSection],
	rebuilt_sections: &[KnowledgePageSectionResponse],
	outputs: &[KnowledgePageRebuildOutput],
) -> Vec<KnowledgePageSectionRebuildState> {
	let output_map = outputs_by_section(outputs);
	let before_by_key = before_sections
		.iter()
		.map(|section| (section.section_key.as_str(), section))
		.collect::<BTreeMap<_, _>>();

	rebuilt_sections
		.iter()
		.map(|section| {
			let output_types =
				output_map.get(section.section_key.as_str()).cloned().unwrap_or_default();
			let lint_finding_types = lint_finding_types_for_outputs(&output_types);
			let state = section_state(
				before_by_key.get(section.section_key.as_str()).copied(),
				section,
				&output_types,
			);

			KnowledgePageSectionRebuildState {
				section_key: section.section_key.clone(),
				heading: section.heading.clone(),
				state,
				output_types,
				lint_finding_types,
			}
		})
		.collect()
}

fn blocked_section_states(
	sections: &[KnowledgePageSection],
	outputs: &[KnowledgePageRebuildOutput],
) -> Vec<KnowledgePageSectionRebuildState> {
	let output_map = outputs_by_section(outputs);

	sections
		.iter()
		.map(|section| {
			let output_types =
				output_map.get(section.section_key.as_str()).cloned().unwrap_or_default();
			let lint_finding_types = lint_finding_types_for_outputs(&output_types);
			let state = if output_types.iter().any(|kind| kind == "missing_citation") {
				"blocked"
			} else if output_types.iter().any(|kind| kind == "stale_section") {
				"stale"
			} else {
				"blocked"
			};

			KnowledgePageSectionRebuildState {
				section_key: section.section_key.clone(),
				heading: section.heading.clone(),
				state: state.to_string(),
				output_types,
				lint_finding_types,
			}
		})
		.collect()
}

fn section_state(
	before: Option<&KnowledgePageSection>,
	after: &KnowledgePageSectionResponse,
	output_types: &[String],
) -> String {
	if output_types.iter().any(|kind| kind == "missing_citation") {
		return "blocked".to_string();
	}
	if before.is_some_and(|section| section.content_hash != after.content_hash)
		|| output_types.iter().any(|kind| kind == "changed_claim" || kind == "conflict")
	{
		return "changed".to_string();
	}

	if output_types.iter().any(|kind| kind == "stale_section") {
		return "stale".to_string();
	}

	"unchanged".to_string()
}

fn successful_rebuild_state(
	diff: Option<&Value>,
	outputs: &[KnowledgePageRebuildOutput],
) -> String {
	if diff_content_changed(diff) {
		return "changed".to_string();
	}

	if outputs.iter().any(|output| output.output_type == "stale_section") {
		return "stale".to_string();
	}

	"unchanged".to_string()
}

fn memory_candidates_for_page(
	page: &KnowledgePageResponse,
	outputs: &[KnowledgePageRebuildOutput],
) -> Vec<KnowledgeDeltaMemoryCandidate> {
	let reasons = candidate_reasons_by_section(outputs);

	page.sections
		.iter()
		.filter_map(|section| {
			let reason = reasons.get(section.section_key.as_str())?;

			memory_candidate_for_section(page, section, reason.as_str())
		})
		.collect()
}

fn memory_candidate_for_section(
	page: &KnowledgePageResponse,
	section: &KnowledgePageSectionResponse,
	reason: &str,
) -> Option<KnowledgeDeltaMemoryCandidate> {
	let source_refs = page
		.source_refs
		.iter()
		.filter(|source_ref| source_ref.section_id == Some(section.section_id))
		.filter_map(|source_ref| consolidation_input_ref(source_ref, page, section, reason))
		.collect::<Vec<_>>();

	if source_refs.is_empty() {
		return None;
	}

	let source_snapshot = candidate_source_snapshot(page, section, reason, &source_refs);
	let diff = candidate_diff(page, section, reason);
	let proposed_payload = candidate_proposed_payload(page, section, reason);

	Some(KnowledgeDeltaMemoryCandidate {
		reason: reason.to_string(),
		page_id: page.page.page_id,
		section_id: section.section_id,
		section_key: section.section_key.clone(),
		source_refs,
		source_snapshot,
		diff,
		proposed_payload,
	})
}

fn consolidation_input_ref(
	source_ref: &KnowledgePageSourceRefResponse,
	page: &KnowledgePageResponse,
	section: &KnowledgePageSectionResponse,
	reason: &str,
) -> Option<ConsolidationInputRef> {
	let kind = consolidation_source_kind(source_ref.source_kind.as_str())?;

	Some(ConsolidationInputRef {
		kind,
		id: source_ref.source_id,
		snapshot: ConsolidationSourceSnapshot {
			status: source_ref.source_status.clone(),
			updated_at: source_ref.source_updated_at,
			content_hash: source_ref.source_content_hash.clone(),
			embedding_version: None,
			trace_version: None,
			source_ref: source_ref.source_snapshot.clone(),
			metadata: serde_json::json!({
				"schema": "elf.knowledge_delta.source_ref/v1",
				"reason": reason,
				"page_id": page.page.page_id,
				"page_kind": page.page.page_kind,
				"page_key": page.page.page_key,
				"section_id": section.section_id,
				"section_key": section.section_key,
			}),
		},
	})
}

fn consolidation_source_kind(source_kind: &str) -> Option<ConsolidationSourceKind> {
	match KnowledgeSourceKind::parse(source_kind)? {
		KnowledgeSourceKind::Doc => Some(ConsolidationSourceKind::Doc),
		KnowledgeSourceKind::DocChunk => Some(ConsolidationSourceKind::DocChunk),
		KnowledgeSourceKind::Note => Some(ConsolidationSourceKind::Note),
		KnowledgeSourceKind::Event => Some(ConsolidationSourceKind::Event),
		KnowledgeSourceKind::Relation | KnowledgeSourceKind::Proposal => None,
	}
}

fn candidate_source_snapshot(
	page: &KnowledgePageResponse,
	section: &KnowledgePageSectionResponse,
	reason: &str,
	source_refs: &[ConsolidationInputRef],
) -> Value {
	serde_json::json!({
		"schema": "elf.knowledge_delta.source_snapshot/v1",
		"reason": reason,
		"page": {
			"page_id": page.page.page_id,
			"page_kind": page.page.page_kind,
			"page_key": page.page.page_key,
			"content_hash": page.page.content_hash,
			"rebuild_source_hash": page.page.rebuild_source_hash,
			"previous_version_diff": page.page.previous_version_diff,
		},
		"section": {
			"section_id": section.section_id,
			"section_key": section.section_key,
			"heading": section.heading,
			"content_hash": section.content_hash,
			"citation_count": section.citation_count,
			"source_ref_count": section.source_ref_count,
		},
		"source_ref_count": source_refs.len(),
		"source_mutation_allowed": false,
	})
}

fn candidate_diff(
	page: &KnowledgePageResponse,
	section: &KnowledgePageSectionResponse,
	reason: &str,
) -> ConsolidationProposalDiff {
	ConsolidationProposalDiff {
		summary: format!(
			"Create a reviewable memory candidate for knowledge page '{}' section '{}' because {reason}.",
			page.page.page_key, section.section_key
		),
		before: serde_json::json!({
			"page_id": page.page.page_id,
			"section_id": section.section_id,
			"previous_version_diff": page.page.previous_version_diff,
		}),
		after: serde_json::json!({
			"target": "derived_note",
			"reason": reason,
			"page_id": page.page.page_id,
			"section_id": section.section_id,
			"section_key": section.section_key,
		}),
	}
}

fn candidate_proposed_payload(
	page: &KnowledgePageResponse,
	section: &KnowledgePageSectionResponse,
	reason: &str,
) -> Value {
	let text = truncate_chars(
		format!(
			"Plan: Review knowledge page {} section {} because source changes produced a {reason} delta.",
			page.page.page_key, section.section_key
		)
		.as_str(),
		220,
	);

	serde_json::json!({
		"type": "plan",
		"key": format!(
			"knowledge_delta_{}_{}",
			page.page.page_key.replace('-', "_"),
			section.section_key.replace('-', "_")
		),
		"text": text,
		"scope": "project_shared",
		"importance": 0.65,
		"confidence": 0.72,
		"source_ref": {
			"schema": "elf.knowledge_delta/v1",
			"reason": reason,
			"page_id": page.page.page_id,
			"section_id": section.section_id,
			"page_key": page.page.page_key,
			"section_key": section.section_key,
			"source_mutation_allowed": false,
		}
	})
}

fn candidate_proposal_input(
	candidate: &KnowledgeDeltaMemoryCandidate,
) -> ConsolidationProposalInput {
	ConsolidationProposalInput {
		proposal_kind: "knowledge_delta_memory_candidate".to_string(),
		apply_intent: ConsolidationApplyIntent::CreateDerivedNote,
		source_refs: candidate.source_refs.clone(),
		source_snapshot: candidate.source_snapshot.clone(),
		lineage: ConsolidationLineage {
			source_refs: candidate.source_refs.clone(),
			parent_run_id: None,
			parent_proposal_ids: Vec::new(),
		},
		confidence: 0.72,
		unsupported_claim_flags: Vec::new(),
		markers: candidate_markers(candidate),
		diff: candidate.diff.clone(),
		target_ref: empty_object(),
		proposed_payload: candidate.proposed_payload.clone(),
	}
}

fn candidate_markers(candidate: &KnowledgeDeltaMemoryCandidate) -> ConsolidationMarkers {
	let marker = ConsolidationMarker {
		severity: ConsolidationMarkerSeverity::Medium,
		message: format!(
			"Knowledge delta '{}' requires reviewer confirmation before memory promotion.",
			candidate.reason
		),
		source: candidate.source_refs.first().cloned(),
	};

	if candidate.reason == "conflict" {
		ConsolidationMarkers { contradictions: vec![marker], staleness: Vec::new() }
	} else {
		ConsolidationMarkers { contradictions: Vec::new(), staleness: vec![marker] }
	}
}

fn candidate_run_input_refs(
	candidates: &[KnowledgeDeltaMemoryCandidate],
) -> Vec<ConsolidationInputRef> {
	let mut seen = BTreeSet::new();
	let mut out = Vec::new();

	for source_ref in candidates.iter().flat_map(|candidate| &candidate.source_refs) {
		if seen.insert((source_ref.kind.as_str().to_string(), source_ref.id)) {
			out.push(source_ref.clone());
		}
	}

	out
}

fn knowledge_delta_source_snapshot(
	changed_sources: &[KnowledgePageChangedSource],
	candidates: &[KnowledgeDeltaMemoryCandidate],
) -> Value {
	serde_json::json!({
		"schema": "elf.knowledge_delta.run_source_snapshot/v1",
		"changed_sources": changed_sources,
		"candidate_count": candidates.len(),
		"candidate_reasons": candidates
			.iter()
			.map(|candidate| candidate.reason.clone())
			.collect::<Vec<_>>(),
		"source_mutation_allowed": false,
	})
}

fn proposal_run_summary(
	created: ConsolidationRunCreateResponse,
	proposal_count: usize,
) -> KnowledgePageProposalRunSummary {
	KnowledgePageProposalRunSummary {
		run_id: created.run.run_id,
		job_id: created.job_id,
		proposal_count,
		review_surface: "consolidation_proposals".to_string(),
	}
}

fn watch_rebuild_summary(
	changed_source_count: usize,
	items: &[KnowledgePageWatchRebuildItem],
	memory_candidate_count: usize,
) -> KnowledgePageWatchRebuildSummary {
	KnowledgePageWatchRebuildSummary {
		changed_source_count,
		affected_page_count: items.len(),
		changed_page_count: items.iter().filter(|item| item.rebuild_state == "changed").count(),
		unchanged_page_count: items.iter().filter(|item| item.rebuild_state == "unchanged").count(),
		stale_page_count: items
			.iter()
			.filter(|item| item.outputs.iter().any(|output| output.output_type == "stale_section"))
			.count(),
		blocked_page_count: items.iter().filter(|item| item.rebuild_state == "blocked").count(),
		memory_candidate_count,
	}
}

fn watch_operator_summary(
	summary: &KnowledgePageWatchRebuildSummary,
	proposal_run: Option<&KnowledgePageProposalRunSummary>,
) -> Vec<String> {
	let mut out = vec![format!(
		"Changed-source rebuild inspected {} sources and {} affected knowledge pages.",
		summary.changed_source_count, summary.affected_page_count
	)];

	out.push(format!(
		"Page states: changed={}, unchanged={}, stale={}, blocked={}.",
		summary.changed_page_count,
		summary.unchanged_page_count,
		summary.stale_page_count,
		summary.blocked_page_count
	));
	out.push(format!(
		"Generated {} reviewable memory candidate proposals; source mutation remains disabled.",
		summary.memory_candidate_count
	));

	if let Some(run) = proposal_run {
		out.push(format!(
			"Queued consolidation run {} with {} proposal payloads for review.",
			run.run_id, run.proposal_count
		));
	}

	out
}

fn page_operator_summary(
	page_key: &str,
	rebuild_state: &str,
	output_count: usize,
	candidate_count: usize,
) -> String {
	format!(
		"Knowledge page '{page_key}' rebuild_state={rebuild_state}, outputs={output_count}, memory_candidates={candidate_count}."
	)
}

fn section_lookup(sections: &[KnowledgePageSection]) -> BTreeMap<Uuid, (String, String)> {
	sections
		.iter()
		.map(|section| (section.section_id, (section.section_key.clone(), section.heading.clone())))
		.collect()
}

fn diff_section_keys(diff: Option<&Value>, key: &str) -> BTreeSet<String> {
	diff.and_then(|value| value.get(key))
		.and_then(Value::as_array)
		.map(|items| items.iter().filter_map(Value::as_str).map(ToString::to_string).collect())
		.unwrap_or_default()
}

fn diff_content_changed(diff: Option<&Value>) -> bool {
	diff.and_then(|value| value.get("content_changed")).and_then(Value::as_bool).unwrap_or(false)
		|| !diff_section_keys(diff, "added_section_keys").is_empty()
		|| !diff_section_keys(diff, "removed_section_keys").is_empty()
		|| !diff_section_keys(diff, "changed_section_keys").is_empty()
}

fn changed_source_set(changed_sources: &[KnowledgePageChangedSource]) -> BTreeSet<(String, Uuid)> {
	changed_sources
		.iter()
		.map(|source| (source.source_kind.as_str().to_string(), source.source_id))
		.collect()
}

fn output_section_keys(
	outputs: &[KnowledgePageRebuildOutput],
	output_type: &str,
) -> BTreeSet<String> {
	outputs
		.iter()
		.filter(|output| output.output_type == output_type)
		.filter_map(|output| output.section_key.clone())
		.collect()
}

fn outputs_by_section(outputs: &[KnowledgePageRebuildOutput]) -> BTreeMap<&str, Vec<String>> {
	let mut map = BTreeMap::<&str, Vec<String>>::new();

	for output in outputs {
		let Some(section_key) = output.section_key.as_deref() else {
			continue;
		};

		map.entry(section_key).or_default().push(output.output_type.clone());
	}
	for values in map.values_mut() {
		values.sort();
		values.dedup();
	}

	map
}

fn lint_finding_types_for_outputs(output_types: &[String]) -> Vec<String> {
	let mut out = output_types
		.iter()
		.filter_map(|output_type| match output_type.as_str() {
			"stale_section" => Some("stale_source_ref".to_string()),
			"missing_citation" => Some("missing_citation".to_string()),
			_ => None,
		})
		.collect::<Vec<_>>();

	out.sort();
	out.dedup();

	out
}

fn candidate_reasons_by_section(outputs: &[KnowledgePageRebuildOutput]) -> BTreeMap<&str, String> {
	let mut reasons = BTreeMap::<&str, String>::new();

	for output in outputs {
		let Some(section_key) = output.section_key.as_deref() else {
			continue;
		};

		match output.output_type.as_str() {
			"conflict" => {
				reasons.insert(section_key, "conflict".to_string());
			},
			"changed_claim" => {
				reasons.entry(section_key).or_insert_with(|| "changed_claim".to_string());
			},
			_ => {},
		}
	}

	reasons
}

fn lint_output_message(output_type: &str, heading: &str) -> String {
	match output_type {
		"stale_section" =>
			format!("Knowledge page section '{heading}' cites a stale or missing source."),
		"missing_citation" =>
			format!("Knowledge page section '{heading}' is missing citation coverage."),
		_ => format!("Knowledge page section '{heading}' needs operator review."),
	}
}

fn default_generate_memory_candidates() -> bool {
	true
}

fn source_snapshots(
	docs: Vec<KnowledgeDocSource>,
	doc_chunks: Vec<KnowledgeDocChunkSource>,
	notes: Vec<KnowledgeNoteSource>,
	events: Vec<KnowledgeEventSource>,
	relations: Vec<KnowledgeRelationSource>,
	proposals: Vec<KnowledgeProposalSource>,
) -> Vec<SourceSnapshot> {
	let mut sources = Vec::new();

	sources.extend(docs.into_iter().map(doc_source_snapshot));
	sources.extend(doc_chunks.into_iter().map(doc_chunk_source_snapshot));
	sources.extend(notes.into_iter().map(note_source_snapshot));
	sources.extend(events.into_iter().map(event_source_snapshot));
	sources.extend(relations.into_iter().map(relation_source_snapshot));
	sources.extend(proposals.into_iter().map(proposal_source_snapshot));
	sources.sort_by_key(source_sort_key);

	sources
}

fn source_refs_by_section(
	source_refs: &[KnowledgePageSourceRef],
) -> HashMap<Uuid, Vec<KnowledgePageSourceRef>> {
	let mut by_section = HashMap::<Uuid, Vec<KnowledgePageSourceRef>>::new();

	for source_ref in source_refs {
		let Some(section_id) = source_ref.section_id else {
			continue;
		};

		by_section.entry(section_id).or_default().push(clone_source_ref(source_ref));
	}

	by_section
}

fn recallable_source_refs(
	source_refs: &[KnowledgePageSourceRef],
	current_source_keys: &BTreeSet<String>,
) -> bool {
	!source_refs.is_empty()
		&& source_refs.iter().all(|source_ref| {
			current_source_keys
				.contains(&current_key(source_ref.source_kind.as_str(), source_ref.source_id))
				&& recallable_source_ref(source_ref)
		})
}

fn source_row_read_allowed(
	owner_agent_id: &str,
	scope: &str,
	requester_agent_id: Option<&str>,
	allowed_scopes: &[String],
	shared_grants: &HashSet<access::SharedSpaceGrantKey>,
) -> bool {
	if !allowed_scopes.iter().any(|allowed_scope| allowed_scope == scope) {
		return false;
	}

	let Some(requester_agent_id) = requester_agent_id else {
		return true;
	};

	if scope == "agent_private" {
		return owner_agent_id == requester_agent_id;
	}
	if !matches!(scope, "project_shared" | "org_shared") {
		return false;
	}
	if owner_agent_id == requester_agent_id {
		return true;
	}

	shared_grants.contains(&access::SharedSpaceGrantKey {
		scope: scope.to_string(),
		space_owner_agent_id: owner_agent_id.to_string(),
	})
}

fn recallable_source_ref(source_ref: &KnowledgePageSourceRef) -> bool {
	let Some(status) = source_ref.source_status.as_deref().map(str::trim) else {
		return false;
	};

	if !matches!(status, "active" | "remember" | "update" | "current" | "historical" | "applied") {
		return false;
	}

	!has_non_recallable_span(&source_ref.source_snapshot)
}

fn has_non_recallable_span(source_snapshot: &Value) -> bool {
	match source_snapshot {
		Value::Object(object) =>
			policy_spans_are_non_recallable(object.get("policy_spans"))
				|| object.get("source_span").is_some_and(span_is_non_recallable)
				|| source_spans_are_non_recallable(object.get("source_spans"))
				|| object.values().any(has_non_recallable_span),
		Value::Array(items) => items.iter().any(has_non_recallable_span),
		_ => false,
	}
}

fn policy_spans_are_non_recallable(policy_spans: Option<&Value>) -> bool {
	match policy_spans {
		Some(Value::Array(spans)) => !spans.is_empty(),
		Some(Value::Null) | None => false,
		Some(_) => true,
	}
}

fn source_spans_are_non_recallable(source_spans: Option<&Value>) -> bool {
	match source_spans {
		Some(Value::Array(spans)) => spans.iter().any(span_is_non_recallable),
		Some(Value::Null) | None => false,
		Some(_) => true,
	}
}

fn span_is_non_recallable(span: &Value) -> bool {
	!matches!(span.get("status").and_then(Value::as_str), Some("captured"))
}

fn cloned_source_refs(
	source_refs: Option<&Vec<KnowledgePageSourceRef>>,
) -> Vec<KnowledgePageSourceRef> {
	source_refs.map(|refs| refs.iter().map(clone_source_ref).collect()).unwrap_or_default()
}

fn clone_source_ref(source_ref: &KnowledgePageSourceRef) -> KnowledgePageSourceRef {
	KnowledgePageSourceRef {
		ref_id: source_ref.ref_id,
		page_id: source_ref.page_id,
		section_id: source_ref.section_id,
		source_kind: source_ref.source_kind.clone(),
		source_id: source_ref.source_id,
		source_status: source_ref.source_status.clone(),
		source_updated_at: source_ref.source_updated_at,
		source_content_hash: source_ref.source_content_hash.clone(),
		source_snapshot: source_ref.source_snapshot.clone(),
		citation_metadata: source_ref.citation_metadata.clone(),
		created_at: source_ref.created_at,
	}
}

fn section_response(
	section: KnowledgePageSection,
	source_refs: Vec<KnowledgePageSourceRef>,
) -> KnowledgePageSectionResponse {
	let citation_count = citation_count(&section.citations);
	let source_ref_count = source_refs.len();
	let source_backlinks =
		source_refs.iter().map(KnowledgePageSectionSourceBacklink::from).collect();

	KnowledgePageSectionResponse {
		citation_count,
		source_ref_count,
		coverage_complete: citation_count > 0 && source_ref_count > 0,
		source_backlinks,
		..KnowledgePageSectionResponse::from(section)
	}
}

fn knowledge_page_search_item(
	row: KnowledgePageSearchRow,
	source_refs: Vec<KnowledgePageSourceRef>,
	query: &str,
) -> KnowledgePageSearchItem {
	let source_ref_count = usize::try_from(row.section_source_ref_count).unwrap_or(0);
	let citation_count = citation_count(&row.citations);
	let lint_summary = KnowledgePageLintSummary {
		error_count: row.lint_error_count,
		warning_count: row.lint_warning_count,
		info_count: row.lint_info_count,
		has_errors: row.lint_error_count > 0,
		has_warnings: row.lint_warning_count > 0,
	};
	let coverage_complete =
		row.source_coverage.get("coverage_complete").and_then(Value::as_bool).unwrap_or(false);
	let trust_state = search_trust_state(&lint_summary, coverage_complete, &row);
	let repair_guidance = search_repair_guidance(&trust_state);
	let previous_version_diff = previous_version_diff_from_metadata(&row.rebuild_metadata);

	KnowledgePageSearchItem {
		result_kind: "knowledge_page_section".to_string(),
		page_id: row.page_id,
		page_kind: row.page_kind,
		page_key: row.page_key,
		title: row.title,
		status: row.status,
		section_id: row.section_id,
		section_key: row.section_key,
		heading: row.heading,
		role: row.role,
		snippet: snippet_for_query(row.content.as_str(), query, SEARCH_SNIPPET_CHARS),
		citations: sanitize_search_citations(row.citations),
		citation_count,
		source_ref_count,
		source_refs: source_refs.into_iter().map(search_source_ref_response).collect(),
		source_coverage: row.source_coverage,
		rebuild_metadata: row.rebuild_metadata,
		previous_version_diff,
		lint_summary,
		trust_state,
		derived_notice:
				"Derived knowledge page snippet. Verify cited source documents, spans, memory notes, events, relations, or proposals before treating it as authoritative."
					.to_string(),
		repair_guidance,
		updated_at: row.page_updated_at,
		rebuilt_at: row.rebuilt_at,
	}
}

fn search_source_ref_response(
	source_ref: KnowledgePageSourceRef,
) -> KnowledgePageSourceRefResponse {
	let mut response = KnowledgePageSourceRefResponse::from(source_ref);

	if response.source_kind == KnowledgeSourceKind::Proposal.as_str() {
		response.source_snapshot = sanitize_proposal_snapshot(&response.source_snapshot);
	}

	response
}

fn sanitize_search_citations(citations: Value) -> Value {
	let Value::Array(citations) = citations else {
		return citations;
	};

	Value::Array(citations.into_iter().map(sanitize_search_citation).collect())
}

fn sanitize_search_citation(mut citation: Value) -> Value {
	let is_proposal = citation
		.get("source_kind")
		.and_then(Value::as_str)
		.is_some_and(|kind| kind == KnowledgeSourceKind::Proposal.as_str());

	if !is_proposal {
		return citation;
	}

	if let Some(object) = citation.as_object_mut()
		&& let Some(source_snapshot) = object.get_mut("source_snapshot")
	{
		*source_snapshot = sanitize_proposal_snapshot(source_snapshot);
	}

	citation
}

fn search_trust_state(
	lint: &KnowledgePageLintSummary,
	coverage_complete: bool,
	row: &KnowledgePageSearchRow,
) -> String {
	if lint.has_errors {
		return "derived_error".to_string();
	}
	if lint.has_warnings || row.unsupported_reason.is_some() {
		return "derived_warning".to_string();
	}

	if !coverage_complete || row.section_source_ref_count == 0 {
		return "derived_low_coverage".to_string();
	}

	"derived_clean".to_string()
}

fn search_repair_guidance(trust_state: &str) -> Option<String> {
	match trust_state {
		"derived_error" => Some(
			"Run knowledge page lint, inspect stale or missing source refs, then rebuild the page from current authoritative sources."
				.to_string(),
		),
		"derived_warning" => Some(
			"Inspect unsupported or stale findings before using this derived snippet; rebuild after source review."
				.to_string(),
		),
		"derived_low_coverage" => Some(
			"Rebuild with complete citations or add source-backed sections before relying on this page."
				.to_string(),
		),
		_ => None,
	}
}

fn build_sections(sources: &[SourceSnapshot]) -> Result<Vec<DraftSection>> {
	let doc_indexes = source_indexes(sources, KnowledgeSourceKind::Doc);
	let doc_chunk_indexes = source_indexes(sources, KnowledgeSourceKind::DocChunk);
	let note_indexes = source_indexes(sources, KnowledgeSourceKind::Note);
	let event_indexes = source_indexes(sources, KnowledgeSourceKind::Event);
	let relation_indexes = source_indexes(sources, KnowledgeSourceKind::Relation);
	let proposal_indexes = source_indexes(sources, KnowledgeSourceKind::Proposal);
	let mut sections = Vec::new();

	push_section(
		&mut sections,
		"source-documents",
		"Source Documents",
		"source_documents",
		sources,
		doc_indexes,
	);
	push_section(
		&mut sections,
		"source-spans",
		"Source Spans",
		"source_spans",
		sources,
		doc_chunk_indexes,
	);
	push_section(
		&mut sections,
		"source-notes",
		"Source Notes",
		"current_truth",
		sources,
		note_indexes,
	);
	push_section(&mut sections, "event-audits", "Event Audits", "history", sources, event_indexes);
	push_section(&mut sections, "relations", "Relations", "relations", sources, relation_indexes);
	push_section(
		&mut sections,
		"reviewed-proposals",
		"Reviewed Proposals",
		"proposals",
		sources,
		proposal_indexes,
	);

	if sections.is_empty() {
		return Err(Error::InvalidRequest {
			message: "knowledge page rebuild did not produce any cited sections".to_string(),
		});
	}

	Ok(sections)
}

fn push_section(
	sections: &mut Vec<DraftSection>,
	section_key: &str,
	heading: &str,
	role: &str,
	sources: &[SourceSnapshot],
	source_indexes: Vec<usize>,
) {
	if source_indexes.is_empty() {
		return;
	}

	let ordinal = i32::try_from(sections.len()).unwrap_or(i32::MAX);
	let content = source_indexes
		.iter()
		.filter_map(|index| sources.get(*index))
		.map(|source| format!("- {}", source.line))
		.collect::<Vec<_>>()
		.join("\n");

	sections.push(DraftSection {
		section_id: Uuid::new_v4(),
		section_key: section_key.to_string(),
		heading: heading.to_string(),
		role: role.to_string(),
		content,
		ordinal,
		source_indexes,
		unsupported_reason: None,
		content_hash: String::new(),
		citations: Value::Array(Vec::new()),
	});
}

fn lint_unsupported_sections(sections: &[DraftSection]) -> Vec<LintDraft> {
	sections
		.iter()
		.filter_map(|section| {
			section.unsupported_reason.as_ref().map(|reason| LintDraft {
				section_id: Some(section.section_id),
				finding_type: "unsupported_claim".to_string(),
				severity: "warning".to_string(),
				source_kind: None,
				source_id: None,
				message: format!("Knowledge page section has unsupported content: {reason}"),
				details: serde_json::json!({
					"section_key": section.section_key,
					"unsupported_reason": reason,
					"repair_guidance": repair_guidance_for_finding_type("unsupported_claim"),
				}),
			})
		})
		.collect()
}

fn lint_page_sections(
	page: &KnowledgePage,
	sections: &[KnowledgePageSection],
	source_refs: &[KnowledgePageSourceRef],
) -> Vec<LintDraft> {
	let source_refs_by_section = source_refs_by_section(source_refs);
	let mut findings = Vec::new();

	for section in sections {
		findings.extend(lint_one_section(section, &source_refs_by_section));
	}

	if !coverage_complete(page.source_coverage.as_object()) {
		findings.push(low_source_coverage_finding(page));
	}

	findings
}

fn lint_one_section(
	section: &KnowledgePageSection,
	source_refs_by_section: &HashMap<Uuid, Vec<KnowledgePageSourceRef>>,
) -> Vec<LintDraft> {
	let citation_count = citation_count(&section.citations);
	let source_ref_count =
		source_refs_by_section.get(&section.section_id).map(Vec::len).unwrap_or_default();
	let mut findings = Vec::new();

	if let Some(reason) = &section.unsupported_reason {
		findings.push(section_finding(
			section,
			"unsupported_claim",
			"warning",
			"Knowledge page section contains unsupported content.",
			serde_json::json!({
				"unsupported_reason": reason,
				"citation_count": citation_count,
				"source_ref_count": source_ref_count,
			}),
		));
	}

	if citation_count == 0 && section.unsupported_reason.is_none() {
		findings.push(section_finding(
			section,
			"missing_citation",
			"error",
			"Knowledge page section has no citations.",
			serde_json::json!({ "source_ref_count": source_ref_count }),
		));
	}
	if source_ref_count == 0 && section.unsupported_reason.is_none() {
		findings.push(section_finding(
			section,
			"missing_source_ref",
			"error",
			"Knowledge page section has no normalized source backlinks.",
			serde_json::json!({ "citation_count": citation_count }),
		));
	}

	findings
}

fn section_finding(
	section: &KnowledgePageSection,
	finding_type: &str,
	severity: &str,
	message: &str,
	details: Value,
) -> LintDraft {
	LintDraft {
		section_id: Some(section.section_id),
		finding_type: finding_type.to_string(),
		severity: severity.to_string(),
		source_kind: None,
		source_id: None,
		message: message.to_string(),
		details: with_repair_guidance(
			details,
			section.section_key.as_str(),
			repair_guidance_for_finding_type(finding_type),
		),
	}
}

fn low_source_coverage_finding(page: &KnowledgePage) -> LintDraft {
	LintDraft {
		section_id: None,
		finding_type: "low_source_coverage".to_string(),
		severity: "warning".to_string(),
		source_kind: None,
		source_id: None,
		message: "Knowledge page source coverage is incomplete.".to_string(),
		details: serde_json::json!({
			"source_coverage": page.source_coverage.clone(),
			"repair_guidance": repair_guidance_for_finding_type("low_source_coverage"),
		}),
	}
}

fn with_repair_guidance(details: Value, section_key: &str, guidance: &str) -> Value {
	let mut object = details.as_object().cloned().unwrap_or_default();

	object.insert("section_key".to_string(), Value::String(section_key.to_string()));
	object.insert("repair_guidance".to_string(), Value::String(guidance.to_string()));

	Value::Object(object)
}

fn coverage_complete(coverage: Option<&Map<String, Value>>) -> bool {
	let Some(coverage) = coverage else {
		return false;
	};
	let source_count = coverage.get("source_count").and_then(Value::as_u64).unwrap_or(0);
	let cited_count = coverage.get("cited_source_count").and_then(Value::as_u64).unwrap_or(0);
	let complete = coverage.get("coverage_complete").and_then(Value::as_bool).unwrap_or(false);

	complete && source_count == cited_count
}

fn citation_count(citations: &Value) -> usize {
	citations.as_array().map(Vec::len).unwrap_or_default()
}

fn source_indexes(sources: &[SourceSnapshot], kind: KnowledgeSourceKind) -> Vec<usize> {
	sources
		.iter()
		.enumerate()
		.filter_map(|(index, source)| (source.kind == kind).then_some(index))
		.collect()
}

fn citations_value(section: &DraftSection, sources: &[SourceSnapshot]) -> Value {
	Value::Array(
		section
			.source_indexes
			.iter()
			.filter_map(|index| sources.get(*index))
			.map(source_citation_value)
			.collect(),
	)
}

fn doc_source_snapshot(row: KnowledgeDocSource) -> SourceSnapshot {
	let title = row.title.clone().unwrap_or_else(|| "Untitled source document".to_string());
	let excerpt = truncate_chars(normalize_whitespace(row.content.as_str()).as_str(), 240);
	let line = format!("[doc:{}] {title}: {excerpt}", row.doc_type);
	let snapshot = serde_json::json!({
		"kind": "doc",
		"doc_id": row.doc_id,
		"agent_id": row.agent_id.clone(),
		"scope": row.scope.clone(),
		"doc_type": row.doc_type.clone(),
		"status": row.status.clone(),
		"title": row.title.clone(),
		"content_bytes": row.content_bytes,
		"content_hash": row.content_hash.clone(),
		"source_ref": row.source_ref.clone(),
		"created_at": row.created_at,
		"updated_at": row.updated_at,
	});

	SourceSnapshot {
		kind: KnowledgeSourceKind::Doc,
		id: row.doc_id,
		status: Some(row.status),
		updated_at: Some(row.updated_at),
		content_hash: Some(row.content_hash),
		snapshot,
		citation_metadata: serde_json::json!({ "section_role": "source_document" }),
		line,
	}
}

fn doc_chunk_source_snapshot(row: KnowledgeDocChunkSource) -> SourceSnapshot {
	let title = row.title.clone().unwrap_or_else(|| "Untitled source document".to_string());
	let excerpt = truncate_chars(normalize_whitespace(row.chunk_text.as_str()).as_str(), 240);
	let span_id = source_span_id(
		row.doc_content_hash.as_str(),
		row.start_offset.max(0) as usize,
		row.end_offset.max(row.start_offset).max(0) as usize,
		"captured",
	);
	let line = format!(
		"[doc_chunk:{}:{}-{}] {title}: {excerpt}",
		row.chunk_index, row.start_offset, row.end_offset
	);
	let source_span = serde_json::json!({
		"schema": "doc_source_span/v1",
		"span_id": span_id,
		"chunk_id": row.chunk_id,
		"status": "captured",
		"reason_code": null,
		"start_offset": row.start_offset,
		"end_offset": row.end_offset,
		"content_hash": row.doc_content_hash.clone(),
		"chunk_hash": row.chunk_hash.clone(),
	});
	let snapshot = serde_json::json!({
		"kind": "doc_chunk",
		"chunk_id": row.chunk_id,
		"doc_id": row.doc_id,
		"agent_id": row.agent_id.clone(),
		"scope": row.scope.clone(),
		"doc_type": row.doc_type.clone(),
		"status": row.status.clone(),
		"title": row.title.clone(),
		"source_ref": row.source_ref.clone(),
		"doc_content_hash": row.doc_content_hash.clone(),
		"doc_updated_at": row.doc_updated_at,
		"chunk_index": row.chunk_index,
		"start_offset": row.start_offset,
		"end_offset": row.end_offset,
		"chunk_hash": row.chunk_hash.clone(),
		"chunk_created_at": row.chunk_created_at,
		"source_span": source_span,
	});

	SourceSnapshot {
		kind: KnowledgeSourceKind::DocChunk,
		id: row.chunk_id,
		status: Some(row.status),
		updated_at: Some(row.doc_updated_at),
		content_hash: Some(row.chunk_hash),
		snapshot,
		citation_metadata: serde_json::json!({
			"section_role": "source_span",
			"doc_id": row.doc_id,
			"span_id": span_id,
			"start_offset": row.start_offset,
			"end_offset": row.end_offset,
		}),
		line,
	}
}

fn note_source_snapshot(row: KnowledgeNoteSource) -> SourceSnapshot {
	let content_hash = hash_text(row.text.as_str());
	let line = format!("{}{}", note_prefix(&row), row.text);
	let snapshot = serde_json::json!({
		"kind": "note",
		"note_id": row.note_id,
		"agent_id": row.agent_id.clone(),
		"scope": row.scope.clone(),
		"type": row.note_type.clone(),
		"key": row.key.clone(),
		"status": row.status.clone(),
		"updated_at": row.updated_at,
		"created_at": row.created_at,
		"expires_at": row.expires_at,
		"embedding_version": row.embedding_version.clone(),
		"content_hash": content_hash,
		"source_ref": row.source_ref.clone(),
		"importance": row.importance,
		"confidence": row.confidence,
	});

	SourceSnapshot {
		kind: KnowledgeSourceKind::Note,
		id: row.note_id,
		status: Some(row.status),
		updated_at: Some(row.updated_at),
		content_hash: Some(content_hash),
		snapshot,
		citation_metadata: serde_json::json!({ "section_role": "source_note" }),
		line,
	}
}

fn event_source_snapshot(row: KnowledgeEventSource) -> SourceSnapshot {
	let content_hash = hash_json_lossy(&row.details);
	let line = format!(
		"add_event audit {} {} for {}{}",
		row.note_op,
		row.policy_decision,
		row.note_type,
		row.note_key.as_ref().map(|key| format!(" key {key}")).unwrap_or_default()
	);
	let snapshot = serde_json::json!({
		"kind": "event",
		"decision_id": row.decision_id,
		"agent_id": row.agent_id.clone(),
		"scope": row.scope.clone(),
		"pipeline": row.pipeline.clone(),
		"note_type": row.note_type.clone(),
		"note_key": row.note_key.clone(),
		"note_id": row.note_id,
		"policy_decision": row.policy_decision.clone(),
		"note_op": row.note_op.clone(),
		"reason_code": row.reason_code.clone(),
		"details_hash": content_hash,
		"ts": row.ts,
	});

	SourceSnapshot {
		kind: KnowledgeSourceKind::Event,
		id: row.decision_id,
		status: Some(row.policy_decision),
		updated_at: Some(row.ts),
		content_hash: Some(content_hash),
		snapshot,
		citation_metadata: serde_json::json!({ "section_role": "event_audit" }),
		line,
	}
}

fn relation_source_snapshot(row: KnowledgeRelationSource) -> SourceSnapshot {
	let object = row.object_entity.clone().or(row.object_value.clone()).unwrap_or_default();
	let temporal_status = if row.valid_to.is_some() { "historical" } else { "current" };
	let line = format!("{} {} {} ({temporal_status}).", row.subject, row.predicate, object);
	let content_hash = hash_text(line.as_str());
	let snapshot = serde_json::json!({
		"kind": "relation",
		"fact_id": row.fact_id,
		"agent_id": row.agent_id.clone(),
		"scope": row.scope.clone(),
		"subject": { "canonical": row.subject.clone(), "kind": row.subject_kind.clone() },
		"predicate": row.predicate.clone(),
		"object": {
			"entity": row.object_entity.clone(),
			"kind": row.object_kind.clone(),
			"value": row.object_value.clone()
		},
		"valid_from": row.valid_from,
		"valid_to": row.valid_to,
		"updated_at": row.updated_at,
		"content_hash": content_hash,
		"evidence_notes": row.evidence_notes.clone(),
	});

	SourceSnapshot {
		kind: KnowledgeSourceKind::Relation,
		id: row.fact_id,
		status: Some(temporal_status.to_string()),
		updated_at: Some(row.updated_at),
		content_hash: Some(content_hash),
		snapshot,
		citation_metadata: serde_json::json!({ "section_role": "relation_fact" }),
		line,
	}
}

fn proposal_source_snapshot(row: KnowledgeProposalSource) -> SourceSnapshot {
	let content_hash = hash_json_lossy(&serde_json::json!({
		"diff": row.diff.clone(),
		"proposed_payload": row.proposed_payload.clone(),
		"review_state": row.review_state.clone(),
	}));
	let line = format!("Applied proposal {}", row.proposal_kind);
	let snapshot = sanitize_proposal_snapshot(&serde_json::json!({
		"kind": "proposal",
		"proposal_id": row.proposal_id,
		"run_id": row.run_id,
		"agent_id": row.agent_id.clone(),
		"proposal_kind": row.proposal_kind.clone(),
		"apply_intent": row.apply_intent.clone(),
		"review_state": row.review_state.clone(),
		"source_refs": row.source_refs.clone(),
		"source_snapshot": row.source_snapshot.clone(),
		"lineage": row.lineage.clone(),
		"diff": row.diff.clone(),
		"confidence": row.confidence,
		"unsupported_claim_flags": row.unsupported_claim_flags.clone(),
		"contradiction_markers": row.contradiction_markers.clone(),
		"staleness_markers": row.staleness_markers.clone(),
		"target_ref": row.target_ref.clone(),
		"proposed_payload_hash": content_hash,
		"updated_at": row.updated_at,
	}));

	SourceSnapshot {
		kind: KnowledgeSourceKind::Proposal,
		id: row.proposal_id,
		status: Some(row.review_state),
		updated_at: Some(row.updated_at),
		content_hash: Some(content_hash),
		snapshot,
		citation_metadata: serde_json::json!({ "section_role": "reviewed_proposal" }),
		line,
	}
}

fn sanitize_proposal_snapshot(source_snapshot: &Value) -> Value {
	let Some(object) = source_snapshot.as_object() else {
		return serde_json::json!({
			"kind": "proposal",
			"sanitized": true,
			"source_visibility": "proposal_metadata_only",
		});
	};
	let nested_source_count =
		object.get("source_refs").and_then(Value::as_array).map(Vec::len).unwrap_or_default();
	let mut sanitized = Map::new();

	for key in [
		"kind",
		"proposal_id",
		"run_id",
		"agent_id",
		"proposal_kind",
		"apply_intent",
		"review_state",
		"confidence",
		"proposed_payload_hash",
		"updated_at",
	] {
		if let Some(value) = object.get(key) {
			sanitized.insert(key.to_string(), value.clone());
		}
	}

	sanitized.insert("sanitized".to_string(), Value::Bool(true));
	sanitized.insert(
		"source_visibility".to_string(),
		Value::String("proposal_metadata_only".to_string()),
	);
	sanitized.insert(
		"omitted_fields".to_string(),
		serde_json::json!([
			"source_refs",
			"source_snapshot",
			"lineage",
			"diff",
			"unsupported_claim_flags",
			"contradiction_markers",
			"staleness_markers",
			"target_ref"
		]),
	);
	sanitized.insert(
		"nested_source_ref_count".to_string(),
		Value::Number(Number::from(nested_source_count)),
	);

	Value::Object(sanitized)
}

fn source_citation_value(source: &SourceSnapshot) -> Value {
	serde_json::json!({
		"source_kind": source.kind.as_str(),
		"source_id": source.id,
		"source_status": source.status.clone(),
		"source_updated_at": source.updated_at,
		"source_content_hash": source.content_hash.clone(),
		"source_snapshot": source.snapshot.clone(),
		"citation_metadata": source.citation_metadata.clone(),
	})
}

fn source_snapshot_value(sources: &[SourceSnapshot]) -> Value {
	serde_json::json!({
		"schema": KNOWLEDGE_PAGE_CONTRACT_SCHEMA_V1,
		"sources": sources.iter().map(source_citation_value).collect::<Vec<_>>(),
	})
}

fn source_coverage_value(
	page_kind: KnowledgePageKind,
	page_key: &str,
	sections: &[DraftSection],
	sources: &[SourceSnapshot],
) -> Value {
	let cited = sections
		.iter()
		.flat_map(|section| section.source_indexes.iter().copied())
		.collect::<BTreeSet<_>>();
	let counts = source_counts(sources);

	serde_json::json!({
		"schema": KNOWLEDGE_PAGE_SOURCE_COVERAGE_SCHEMA_V1,
		"page_kind": page_kind.as_str(),
		"page_key": page_key,
		"source_counts": counts,
		"source_count": sources.len(),
		"cited_source_count": cited.len(),
		"section_count": sections.len(),
		"unsupported_section_count": sections.iter().filter(|section| section.unsupported_reason.is_some()).count(),
		"coverage_complete": cited.len() == sources.len(),
	})
}

fn source_counts(sources: &[SourceSnapshot]) -> Value {
	let mut counts = BTreeMap::<&str, usize>::new();

	for source in sources {
		*counts.entry(source.kind.as_str()).or_insert(0) += 1;
	}

	serde_json::json!(counts)
}

fn rebuild_metadata(
	source_hash: &str,
	provider_metadata: &Value,
	req: &KnowledgePageRebuildRequest,
) -> Value {
	let llm_derived =
		provider_metadata.get("llm_derived").and_then(Value::as_bool).unwrap_or(false);

	serde_json::json!({
		"schema": KNOWLEDGE_PAGE_REBUILD_SCHEMA_V1,
		"source_snapshot_hash": source_hash,
		"deterministic": !llm_derived,
		"provider_metadata": provider_metadata,
		"generated_by": {
			"schema": "elf.knowledge_page.generated_by/v1",
			"runtime": "ElfService::knowledge_page_rebuild",
			"actor_agent_id": req.agent_id,
			"mode": if llm_derived { "provider_metadata_declared_llm" } else { "deterministic_service" },
			"source_input_counts": {
				"doc": req.doc_ids.len(),
				"doc_chunk": req.doc_chunk_ids.len(),
				"note": req.note_ids.len(),
				"event": req.event_ids.len(),
				"relation": req.relation_ids.len(),
				"proposal": req.proposal_ids.len(),
			},
		},
		"memory_candidate_policy": {
			"schema": "elf.knowledge_page.memory_candidate_policy/v1",
			"review_required": true,
			"review_surface": "consolidation_proposals",
			"proposal_contract_schema": "elf.consolidation/v1",
			"allowed_apply_intents": ["create_derived_note", "update_derived_note"],
			"direct_memory_ledger_mutation_allowed": false,
			"source_mutation_allowed": false,
		},
		"allowed_variance": if llm_derived {
			serde_json::json!(["LLM-derived page text may vary; provider metadata records the nondeterministic input path."])
		} else {
			serde_json::json!([])
		},
	})
}

fn rebuild_metadata_with_previous_version_diff(
	mut metadata: Value,
	diff: Value,
	version_identity: Value,
) -> Value {
	let Some(object) = metadata.as_object_mut() else {
		return serde_json::json!({
			PREVIOUS_VERSION_DIFF_KEY: diff,
			"version_identity": version_identity,
		});
	};

	object.insert(PREVIOUS_VERSION_DIFF_KEY.to_string(), diff);
	object.insert("version_identity".to_string(), version_identity);

	metadata
}

fn previous_version_diff_from_metadata(metadata: &Value) -> Option<Value> {
	metadata
		.get(PREVIOUS_VERSION_DIFF_KEY)
		.filter(|diff| diff.as_object().is_some_and(|object| !object.is_empty()))
		.cloned()
}

fn previous_version_diff_value(
	previous: Option<&KnowledgePage>,
	previous_sections: &[KnowledgePageSection],
	new_title: &str,
	new_source_hash: &str,
	new_content_hash: &str,
	new_sections: &[DraftSection],
) -> Value {
	let Some(previous) = previous else {
		return serde_json::json!({
			"schema": KNOWLEDGE_PAGE_VERSION_DIFF_SCHEMA_V1,
			"available": false,
			"reason": "no_previous_version",
			"summary": "Initial rebuild; no previous knowledge page version exists.",
			"source_mutation_allowed": false,
		});
	};
	let previous_by_key = previous_sections
		.iter()
		.map(|section| (section.section_key.as_str(), section))
		.collect::<BTreeMap<_, _>>();
	let new_by_key = new_sections
		.iter()
		.map(|section| (section.section_key.as_str(), section))
		.collect::<BTreeMap<_, _>>();
	let previous_keys = previous_by_key.keys().copied().collect::<BTreeSet<_>>();
	let new_keys = new_by_key.keys().copied().collect::<BTreeSet<_>>();
	let added_section_keys = sorted_strings(new_keys.difference(&previous_keys).copied());
	let removed_section_keys = sorted_strings(previous_keys.difference(&new_keys).copied());
	let mut changed_section_keys = Vec::new();
	let mut unchanged_section_keys = Vec::new();

	for key in previous_keys.intersection(&new_keys).copied() {
		let previous_section = previous_by_key[key];
		let new_section = new_by_key[key];

		if previous_section.content_hash == new_section.content_hash
			&& previous_section.heading == new_section.heading
			&& previous_section.role == new_section.role
			&& previous_section.unsupported_reason == new_section.unsupported_reason
		{
			unchanged_section_keys.push(key.to_string());
		} else {
			changed_section_keys.push(key.to_string());
		}
	}

	let title_changed = previous.title != new_title;
	let source_changed = previous.rebuild_source_hash != new_source_hash;
	let content_changed = previous.content_hash != new_content_hash;
	let summary = version_diff_summary(
		title_changed,
		source_changed,
		content_changed,
		added_section_keys.len(),
		removed_section_keys.len(),
		changed_section_keys.len(),
	);

	serde_json::json!({
		"schema": KNOWLEDGE_PAGE_VERSION_DIFF_SCHEMA_V1,
		"available": true,
		"previous_page_id": previous.page_id,
		"previous_content_hash": previous.content_hash,
		"new_content_hash": new_content_hash,
		"previous_source_hash": previous.rebuild_source_hash,
		"new_source_hash": new_source_hash,
		"title_changed": title_changed,
		"source_changed": source_changed,
		"content_changed": content_changed,
		"section_added_count": added_section_keys.len(),
		"section_removed_count": removed_section_keys.len(),
		"section_changed_count": changed_section_keys.len(),
		"section_unchanged_count": unchanged_section_keys.len(),
		"added_section_keys": added_section_keys,
		"removed_section_keys": removed_section_keys,
		"changed_section_keys": changed_section_keys,
		"unchanged_section_keys": unchanged_section_keys,
		"source_mutation_allowed": false,
		"summary": summary,
	})
}

fn version_identity_value(
	page_kind: KnowledgePageKind,
	page_key: &str,
	source_hash: &str,
	content_hash: &str,
	sections: &[DraftSection],
) -> Value {
	serde_json::json!({
		"schema": "elf.knowledge_page.version_identity/v1",
		"contract_schema": KNOWLEDGE_PAGE_CONTRACT_SCHEMA_V1,
		"page_kind": page_kind.as_str(),
		"page_key": page_key,
		"source_snapshot_hash": source_hash,
		"content_hash": content_hash,
		"section_hashes": sections
			.iter()
			.map(|section| {
				serde_json::json!({
					"section_key": section.section_key.clone(),
					"content_hash": section.content_hash.clone(),
				})
			})
			.collect::<Vec<_>>(),
		"source_mutation_allowed": false,
	})
}

fn sorted_strings<'a>(items: impl Iterator<Item = &'a str>) -> Vec<String> {
	let mut out = items.map(ToString::to_string).collect::<Vec<_>>();

	out.sort();

	out
}

fn version_diff_summary(
	title_changed: bool,
	source_changed: bool,
	content_changed: bool,
	added: usize,
	removed: usize,
	changed: usize,
) -> String {
	if !title_changed
		&& !source_changed
		&& !content_changed
		&& added == 0
		&& removed == 0
		&& changed == 0
	{
		return "No page-level or section-level changes from the previous rebuild.".to_string();
	}

	format!(
		"Previous rebuild diff: title_changed={title_changed}, source_changed={source_changed}, content_changed={content_changed}, sections added={added}, removed={removed}, changed={changed}."
	)
}

fn content_hash_rebuild_metadata(rebuild_metadata: &Value) -> Value {
	let Some(object) = rebuild_metadata.as_object() else {
		return rebuild_metadata.clone();
	};
	let mut stable = object.clone();

	stable.remove(PREVIOUS_VERSION_DIFF_KEY);
	stable.remove("generated_by");
	stable.remove("memory_candidate_policy");
	stable.remove("version_identity");

	Value::Object(stable)
}

fn section_hash_payload(section: &DraftSection) -> Value {
	serde_json::json!({
		"section_key": section.section_key.clone(),
		"heading": section.heading.clone(),
		"role": section.role.clone(),
		"content": section.content.clone(),
		"citations": section.citations.clone(),
		"unsupported_reason": section.unsupported_reason.clone(),
	})
}

fn page_content_hash(
	title: &str,
	sections: &[DraftSection],
	source_coverage: &Value,
	rebuild_metadata: &Value,
) -> Result<String> {
	let stable_rebuild_metadata = content_hash_rebuild_metadata(rebuild_metadata);

	hash_json(&serde_json::json!({
		"title": title,
		"sections": sections.iter().map(section_hash_payload).collect::<Vec<_>>(),
		"source_coverage": source_coverage,
		"rebuild_metadata": stable_rebuild_metadata,
	}))
}

fn missing_source_finding(source_ref: &KnowledgePageSourceRef) -> LintDraft {
	LintDraft {
		section_id: source_ref.section_id,
		finding_type: "stale_source_ref".to_string(),
		severity: "error".to_string(),
		source_kind: KnowledgeSourceKind::parse(source_ref.source_kind.as_str()),
		source_id: Some(source_ref.source_id),
		message: "Knowledge page source reference no longer resolves.".to_string(),
		details: serde_json::json!({
			"source_kind": source_ref.source_kind.clone(),
			"source_id": source_ref.source_id,
			"repair_guidance": repair_guidance_for_finding_type("stale_source_ref"),
		}),
	}
}

fn stale_source_finding(
	source_ref: &KnowledgePageSourceRef,
	current: &SourceSnapshot,
) -> LintDraft {
	LintDraft {
		section_id: source_ref.section_id,
		finding_type: "stale_source_ref".to_string(),
		severity: "warning".to_string(),
		source_kind: Some(current.kind),
		source_id: Some(current.id),
		message: "Knowledge page source reference snapshot is stale.".to_string(),
		details: serde_json::json!({
			"stored": {
				"status": source_ref.source_status.clone(),
				"updated_at": source_ref.source_updated_at,
				"content_hash": source_ref.source_content_hash.clone(),
			},
			"current": {
				"status": current.status.clone(),
				"updated_at": current.updated_at,
				"content_hash": current.content_hash.clone(),
			},
			"repair_guidance": repair_guidance_for_finding_type("stale_source_ref"),
		}),
	}
}

fn repair_guidance_for_finding_type(finding_type: &str) -> &'static str {
	match finding_type {
		"stale_source_ref" =>
			"Inspect the stale or missing source, then rebuild the page from current authoritative sources.",
		"unsupported_claim" =>
			"Replace the unsupported section content with source-backed text or rebuild from cited sources.",
		"missing_citation" =>
			"Rebuild the page section with explicit citations or mark the section unsupported with a reason.",
		"missing_source_ref" =>
			"Rebuild the page so each section citation is normalized into knowledge_page_source_refs.",
		"low_source_coverage" =>
			"Rebuild with all intended sources or remove uncited material before relying on this page.",
		_ => "Inspect the finding and rebuild the page after source review.",
	}
}

fn source_changed(source_ref: &KnowledgePageSourceRef, current: &SourceSnapshot) -> bool {
	source_ref.source_status.as_deref() != current.status.as_deref()
		|| source_ref.source_updated_at != current.updated_at
		|| source_ref.source_content_hash.as_deref() != current.content_hash.as_deref()
}

fn snippet_for_query(content: &str, query: &str, max_chars: usize) -> String {
	let normalized = normalize_whitespace(content);
	let query = query.trim();

	if query.is_empty() {
		return truncate_chars(normalized.as_str(), max_chars);
	}

	let lower = normalized.to_ascii_lowercase();
	let lower_query = query.to_ascii_lowercase();
	let Some(byte_idx) = lower.find(lower_query.as_str()) else {
		return truncate_chars(normalized.as_str(), max_chars);
	};
	let before_chars = normalized[..byte_idx].chars().count();
	let start = before_chars.saturating_sub(40);
	let mut snippet: String = normalized.chars().skip(start).take(max_chars).collect();

	if start > 0 {
		snippet = format!("...{snippet}");
	}
	if normalized.chars().count() > start + snippet.chars().count() {
		snippet.push_str("...");
	}

	snippet
}

fn normalize_whitespace(raw: &str) -> String {
	let mut out = String::with_capacity(raw.len());
	let mut prev_space = false;

	for ch in raw.chars() {
		if ch.is_whitespace() {
			if !prev_space {
				out.push(' ');

				prev_space = true;
			}

			continue;
		}

		out.push(ch);

		prev_space = false;
	}

	out.trim().to_string()
}

fn truncate_chars(raw: &str, max_chars: usize) -> String {
	if raw.chars().count() <= max_chars {
		return raw.to_string();
	}

	const TRUNCATION_MARKER: &str = "...";

	let marker_chars = TRUNCATION_MARKER.chars().count();

	if max_chars <= marker_chars {
		return TRUNCATION_MARKER.chars().take(max_chars).collect();
	}

	let truncated_chars = max_chars - marker_chars;
	let mut out = raw.chars().take(truncated_chars).collect::<String>();

	out.push_str(TRUNCATION_MARKER);

	out
}

fn source_sort_key(source: &SourceSnapshot) -> (String, Uuid) {
	(source.kind.as_str().to_string(), source.id)
}

fn source_key(source: &SourceSnapshot) -> String {
	current_key(source.kind.as_str(), source.id)
}

fn current_key(kind: &str, source_id: Uuid) -> String {
	format!("{kind}:{source_id}")
}

fn note_prefix(row: &KnowledgeNoteSource) -> String {
	row.key
		.as_ref()
		.map(|key| format!("[{}:{key}] ", row.note_type))
		.unwrap_or_else(|| format!("[{}] ", row.note_type))
}

fn generated_title(page_kind: KnowledgePageKind, page_key: &str) -> String {
	format!("{} Knowledge Page: {page_key}", title_kind(page_kind))
}

fn title_kind(page_kind: KnowledgePageKind) -> &'static str {
	match page_kind {
		KnowledgePageKind::Project => "Project",
		KnowledgePageKind::Entity => "Entity",
		KnowledgePageKind::Concept => "Concept",
		KnowledgePageKind::Issue => "Issue",
		KnowledgePageKind::Decision => "Decision",
		KnowledgePageKind::Author => "Author",
		KnowledgePageKind::Timeline => "Timeline",
	}
}

fn sorted_unique(ids: &[Uuid]) -> Vec<Uuid> {
	ids.iter().copied().collect::<BTreeSet<_>>().into_iter().collect()
}

fn bounded_limit(limit: Option<u32>) -> i64 {
	limit.map(i64::from).unwrap_or(DEFAULT_LIST_LIMIT).clamp(1, MAX_LIST_LIMIT)
}

fn validate_context(tenant_id: &str, project_id: &str, agent_id: &str) -> Result<()> {
	validate_non_empty("tenant_id", tenant_id)?;
	validate_non_empty("project_id", project_id)?;

	validate_non_empty("agent_id", agent_id)
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<()> {
	if value.trim().is_empty() {
		return Err(Error::InvalidRequest { message: format!("{field} must not be empty.") });
	}

	Ok(())
}

fn validate_object(field: &str, value: &Value) -> Result<()> {
	if matches!(value, Value::Object(_)) {
		Ok(())
	} else {
		Err(Error::InvalidRequest { message: format!("{field} must be a JSON object.") })
	}
}

fn empty_object() -> Value {
	Value::Object(Map::new())
}

fn hash_text(text: &str) -> String {
	blake3::hash(text.as_bytes()).to_hex().to_string()
}

fn hash_json_lossy(value: &Value) -> String {
	serde_json::to_vec(value)
		.map(|raw| blake3::hash(&raw).to_hex().to_string())
		.unwrap_or_else(|_| hash_text(value.to_string().as_str()))
}

fn hash_json(value: &Value) -> Result<String> {
	let raw = serde_json::to_vec(value).map_err(|err| Error::InvalidRequest {
		message: format!("failed to serialize knowledge page payload: {err}"),
	})?;

	Ok(blake3::hash(&raw).to_hex().to_string())
}

fn source_span_id(content_hash: &str, start: usize, end: usize, span_kind: &str) -> Uuid {
	let name = serde_json::json!(["elf-doc-source-span/v1", content_hash, start, end, span_kind])
		.to_string();

	Uuid::new_v5(&Uuid::NAMESPACE_OID, name.as_bytes())
}

async fn replace_page_children(
	tx: &mut Transaction<'_, Postgres>,
	page_id: Uuid,
	sections: &[DraftSection],
	sources: &[SourceSnapshot],
	lint: &[LintDraft],
	now: OffsetDateTime,
) -> Result<()> {
	knowledge::delete_knowledge_page_children(&mut **tx, page_id).await?;

	for section in sections {
		insert_section(tx, page_id, section, now).await?;

		for source_index in &section.source_indexes {
			let source = sources.get(*source_index).ok_or_else(|| Error::InvalidRequest {
				message: "knowledge page section referenced an unknown source".to_string(),
			})?;

			insert_source_ref(tx, page_id, section.section_id, source, now).await?;
		}
	}
	for finding in lint {
		insert_lint_finding(tx, page_id, finding, now).await?;
	}

	Ok(())
}

async fn insert_section(
	tx: &mut Transaction<'_, Postgres>,
	page_id: Uuid,
	section: &DraftSection,
	now: OffsetDateTime,
) -> Result<()> {
	knowledge::insert_knowledge_page_section(
		&mut **tx,
		KnowledgePageSectionInsert {
			section_id: section.section_id,
			page_id,
			section_key: section.section_key.as_str(),
			heading: section.heading.as_str(),
			role: section.role.as_str(),
			content: section.content.as_str(),
			ordinal: section.ordinal,
			citations: &section.citations,
			unsupported_reason: section.unsupported_reason.as_deref(),
			content_hash: section.content_hash.as_str(),
			now,
		},
	)
	.await
	.map_err(Error::from)
}

async fn insert_source_ref(
	tx: &mut Transaction<'_, Postgres>,
	page_id: Uuid,
	section_id: Uuid,
	source: &SourceSnapshot,
	now: OffsetDateTime,
) -> Result<()> {
	knowledge::insert_knowledge_page_source_ref(
		&mut **tx,
		KnowledgePageSourceRefInsert {
			ref_id: Uuid::new_v4(),
			page_id,
			section_id: Some(section_id),
			source_kind: source.kind.as_str(),
			source_id: source.id,
			source_status: source.status.as_deref(),
			source_updated_at: source.updated_at,
			source_content_hash: source.content_hash.as_deref(),
			source_snapshot: &source.snapshot,
			citation_metadata: &source.citation_metadata,
			now,
		},
	)
	.await
	.map_err(Error::from)
}

async fn insert_lint_finding(
	tx: &mut Transaction<'_, Postgres>,
	page_id: Uuid,
	finding: &LintDraft,
	now: OffsetDateTime,
) -> Result<()> {
	knowledge::insert_knowledge_page_lint_finding(
		&mut **tx,
		KnowledgePageLintFindingInsert {
			finding_id: Uuid::new_v4(),
			page_id,
			section_id: finding.section_id,
			finding_type: finding.finding_type.as_str(),
			severity: finding.severity.as_str(),
			source_kind: finding.source_kind.map(KnowledgeSourceKind::as_str),
			source_id: finding.source_id,
			message: finding.message.as_str(),
			details: &finding.details,
			now,
		},
	)
	.await
	.map_err(Error::from)
}

#[cfg(test)]
mod tests {
	use std::{
		collections::{BTreeSet, HashSet},
		slice,
	};

	use crate::{
		access::SharedSpaceGrantKey,
		knowledge::{
			self, DraftSection, KnowledgeDeltaMemoryCandidate, KnowledgePage, KnowledgePageKind,
			KnowledgePageResponse, KnowledgePageSearchRow, KnowledgePageSection,
			KnowledgePageSectionResponse, KnowledgePageSourceRef, KnowledgePageSourceRefResponse,
			KnowledgePageSummary, KnowledgeSourceKind, LintDraft, OffsetDateTime, SourceSnapshot,
			Uuid,
		},
	};
	use elf_domain::consolidation::ConsolidationApplyIntent;

	fn test_source(kind: KnowledgeSourceKind, raw_id: u128, line: &str) -> SourceSnapshot {
		let id = Uuid::from_u128(raw_id);
		let content_hash = knowledge::hash_text(line);

		SourceSnapshot {
			kind,
			id,
			status: Some("active".to_string()),
			updated_at: Some(OffsetDateTime::UNIX_EPOCH),
			content_hash: Some(content_hash.clone()),
			snapshot: serde_json::json!({
				"kind": kind.as_str(),
				"id": id,
				"status": "active",
				"updated_at": OffsetDateTime::UNIX_EPOCH,
				"content_hash": content_hash,
			}),
			citation_metadata: serde_json::json!({ "fixture": "knowledge_unit" }),
			line: line.to_string(),
		}
	}

	fn test_rebuild_request(
		page_kind: KnowledgePageKind,
	) -> knowledge::KnowledgePageRebuildRequest {
		knowledge::KnowledgePageRebuildRequest {
			tenant_id: "tenant".to_string(),
			project_id: "project".to_string(),
			agent_id: "agent".to_string(),
			page_kind,
			page_key: "elf".to_string(),
			title: Some("ELF".to_string()),
			doc_ids: Vec::new(),
			doc_chunk_ids: Vec::new(),
			note_ids: Vec::new(),
			event_ids: Vec::new(),
			relation_ids: Vec::new(),
			proposal_ids: Vec::new(),
			provider_metadata: knowledge::empty_object(),
		}
	}

	#[test]
	fn build_sections_preserves_citations_and_deterministic_hashes() {
		let sources = vec![
			test_source(KnowledgeSourceKind::Doc, 1, "A source document supports the page."),
			test_source(KnowledgeSourceKind::DocChunk, 2, "A source span supports the page."),
			test_source(KnowledgeSourceKind::Note, 3, "A source note supports the page."),
			test_source(KnowledgeSourceKind::Event, 4, "An event audit supports the page."),
			test_source(KnowledgeSourceKind::Relation, 5, "A relation supports the page."),
			test_source(KnowledgeSourceKind::Proposal, 6, "An applied proposal supports the page."),
		];
		let mut first_sections =
			knowledge::build_sections(&sources).expect("sections should build");

		for section in &mut first_sections {
			section.citations = knowledge::citations_value(section, &sources);
			section.content_hash = knowledge::hash_json(&knowledge::section_hash_payload(section))
				.expect("section hash should serialize");
		}

		assert_eq!(first_sections.len(), 6);
		assert!(first_sections.iter().all(|section| {
			section.citations.as_array().is_some_and(|citations| !citations.is_empty())
		}));

		let coverage = knowledge::source_coverage_value(
			KnowledgePageKind::Project,
			"elf",
			&first_sections,
			&sources,
		);
		let request = test_rebuild_request(KnowledgePageKind::Project);
		let metadata =
			knowledge::rebuild_metadata("source-hash", &knowledge::empty_object(), &request);
		let first_hash = knowledge::page_content_hash("ELF", &first_sections, &coverage, &metadata)
			.expect("page hash should serialize");
		let second_hash =
			knowledge::page_content_hash("ELF", &first_sections, &coverage, &metadata)
				.expect("page hash should serialize");

		assert_eq!(coverage["coverage_complete"], true);
		assert_eq!(metadata["deterministic"], true);
		assert_eq!(
			metadata["memory_candidate_policy"]["direct_memory_ledger_mutation_allowed"],
			false
		);
		assert_eq!(first_hash, second_hash);
	}

	#[test]
	fn rebuild_metadata_records_llm_variance() {
		let metadata = knowledge::rebuild_metadata(
			"source-hash",
			&serde_json::json!({
				"llm_derived": true,
				"provider_id": "fixture",
				"model": "fixture-model",
			}),
			&test_rebuild_request(KnowledgePageKind::Timeline),
		);

		assert_eq!(metadata["deterministic"], false);
		assert!(metadata["allowed_variance"].as_array().is_some_and(|items| !items.is_empty()));
		assert_eq!(metadata["provider_metadata"]["provider_id"], "fixture");
		assert_eq!(metadata["generated_by"]["actor_agent_id"], "agent");
	}

	#[test]
	fn generated_titles_cover_author_and_timeline_pages() {
		assert_eq!(
			knowledge::generated_title(KnowledgePageKind::Author, "ada"),
			"Author Knowledge Page: ada"
		);
		assert_eq!(
			knowledge::generated_title(KnowledgePageKind::Timeline, "release-plan"),
			"Timeline Knowledge Page: release-plan"
		);
	}

	#[test]
	fn previous_version_diff_records_delta_without_changing_content_hash() {
		let previous = test_page();
		let previous_section =
			test_section(Uuid::from_u128(10), "source-notes", serde_json::json!([]), None);
		let sections = vec![DraftSection {
			section_id: Uuid::from_u128(12),
			section_key: "source-notes".to_string(),
			heading: "source-notes".to_string(),
			role: "current_truth".to_string(),
			content: "Updated section content.".to_string(),
			ordinal: 0,
			source_indexes: vec![0],
			unsupported_reason: None,
			content_hash: "new-section-hash".to_string(),
			citations: serde_json::json!([{ "source_kind": "note" }]),
		}];
		let request = test_rebuild_request(KnowledgePageKind::Project);
		let base_metadata =
			knowledge::rebuild_metadata("new-source-hash", &knowledge::empty_object(), &request);
		let coverage = serde_json::json!({ "coverage_complete": true });
		let hash_without_diff =
			knowledge::page_content_hash("ELF", &sections, &coverage, &base_metadata)
				.expect("stable hash should serialize");
		let diff = knowledge::previous_version_diff_value(
			Some(&previous),
			&[previous_section],
			"ELF",
			"new-source-hash",
			hash_without_diff.as_str(),
			&sections,
		);
		let version_identity = knowledge::version_identity_value(
			KnowledgePageKind::Project,
			"elf",
			"new-source-hash",
			hash_without_diff.as_str(),
			&sections,
		);
		let metadata_with_diff = knowledge::rebuild_metadata_with_previous_version_diff(
			base_metadata,
			diff.clone(),
			version_identity,
		);
		let hash_with_diff =
			knowledge::page_content_hash("ELF", &sections, &coverage, &metadata_with_diff)
				.expect("hash should ignore previous-version diff metadata");

		assert_eq!(hash_without_diff, hash_with_diff);
		assert_eq!(diff["schema"], "elf.knowledge_page.version_diff/v1");
		assert_eq!(diff["available"], true);
		assert_eq!(diff["source_mutation_allowed"], false);
		assert_eq!(diff["section_changed_count"], 1);
		assert_eq!(
			knowledge::previous_version_diff_from_metadata(&metadata_with_diff)
				.expect("diff should be extractable")["section_changed_count"],
			1
		);
		assert_eq!(
			metadata_with_diff["version_identity"]["schema"],
			"elf.knowledge_page.version_identity/v1"
		);
	}

	#[test]
	fn stale_source_comparison_detects_changed_snapshot() {
		let source_id = Uuid::from_u128(42);
		let stored = KnowledgePageSourceRef {
			ref_id: Uuid::from_u128(1),
			page_id: Uuid::from_u128(2),
			section_id: Some(Uuid::from_u128(3)),
			source_kind: "note".to_string(),
			source_id,
			source_status: Some("active".to_string()),
			source_updated_at: Some(OffsetDateTime::UNIX_EPOCH),
			source_content_hash: Some("old-hash".to_string()),
			source_snapshot: serde_json::json!({}),
			citation_metadata: serde_json::json!({}),
			created_at: OffsetDateTime::UNIX_EPOCH,
		};
		let current = SourceSnapshot {
			kind: KnowledgeSourceKind::Note,
			id: source_id,
			status: Some("active".to_string()),
			updated_at: Some(OffsetDateTime::UNIX_EPOCH),
			content_hash: Some("new-hash".to_string()),
			snapshot: serde_json::json!({}),
			citation_metadata: serde_json::json!({}),
			line: "Updated note source.".to_string(),
		};
		let finding = knowledge::stale_source_finding(&stored, &current);

		assert!(knowledge::source_changed(&stored, &current));
		assert_eq!(finding.finding_type, "stale_source_ref");
		assert_eq!(finding.source_kind, Some(KnowledgeSourceKind::Note));
		assert_eq!(finding.source_id, Some(source_id));
	}

	#[test]
	fn watch_rebuild_outputs_cover_source_update_and_stale_page() {
		let section_id = Uuid::from_u128(50);
		let source_id = Uuid::from_u128(51);
		let section = test_section(
			section_id,
			"source-notes",
			serde_json::json!([{ "source_kind": "note", "source_id": source_id }]),
			None,
		);
		let source_ref = test_source_ref_for(section_id, source_id, "old-hash");
		let lint = vec![LintDraft {
			section_id: Some(section_id),
			finding_type: "stale_source_ref".to_string(),
			severity: "warning".to_string(),
			source_kind: Some(KnowledgeSourceKind::Note),
			source_id: Some(source_id),
			message: "Knowledge page source reference snapshot is stale.".to_string(),
			details: serde_json::json!({ "stored": "old", "current": "new" }),
		}];
		let diff = serde_json::json!({
			"available": true,
			"content_changed": true,
			"changed_section_keys": ["source-notes"]
		});
		let changed_sources = vec![knowledge::KnowledgePageChangedSource {
			source_kind: KnowledgeSourceKind::Note,
			source_id,
		}];
		let outputs = knowledge::rebuild_outputs(
			&[section],
			&[source_ref],
			&lint,
			Some(&diff),
			&changed_sources,
		);
		let output_types =
			outputs.iter().map(|output| output.output_type.as_str()).collect::<Vec<_>>();

		assert!(output_types.contains(&"stale_section"));
		assert!(output_types.contains(&"changed_claim"));
		assert!(output_types.contains(&"conflict"));
		assert!(output_types.contains(&"changed_source"));
	}

	#[test]
	fn memory_candidate_uses_reviewable_consolidation_proposal_contract() {
		let section_id = Uuid::from_u128(60);
		let source_id = Uuid::from_u128(61);
		let page = test_page_response(section_id, source_id);
		let outputs = vec![knowledge::KnowledgePageRebuildOutput {
			output_type: "changed_claim".to_string(),
			severity: "info".to_string(),
			section_key: Some("source-notes".to_string()),
			source_kind: Some("note".to_string()),
			source_id: Some(source_id),
			message: "Changed section.".to_string(),
			details: serde_json::json!({ "reason": "source_update" }),
		}];
		let candidates = knowledge::memory_candidates_for_page(&page, &outputs);

		assert_eq!(candidates.len(), 1);

		assert_candidate_is_reviewable(&candidates[0]);

		let proposal = knowledge::candidate_proposal_input(&candidates[0]);

		assert_eq!(proposal.apply_intent, ConsolidationApplyIntent::CreateDerivedNote);
		assert_eq!(proposal.source_refs.len(), 1);
		assert_eq!(proposal.proposed_payload["source_ref"]["source_mutation_allowed"], false);
		assert_eq!(proposal.proposed_payload["source_ref"]["reason"], "changed_claim");
		assert!(!proposal.markers.staleness.is_empty());
	}

	#[test]
	fn lint_page_sections_detects_unsupported_missing_and_low_coverage() {
		let page = test_page();
		let unsupported = test_section(
			Uuid::from_u128(10),
			"unsupported",
			serde_json::json!([]),
			Some("No source supports this claim.".to_string()),
		);
		let missing = test_section(Uuid::from_u128(11), "missing", serde_json::json!([]), None);
		let findings = knowledge::lint_page_sections(&page, &[unsupported, missing], &[]);
		let finding_types =
			findings.iter().map(|finding| finding.finding_type.as_str()).collect::<Vec<_>>();

		assert!(finding_types.contains(&"unsupported_claim"));
		assert!(finding_types.contains(&"missing_citation"));
		assert!(finding_types.contains(&"missing_source_ref"));
		assert!(finding_types.contains(&"low_source_coverage"));
		assert!(findings.iter().all(|finding| {
			finding
				.details
				.get("repair_guidance")
				.and_then(serde_json::Value::as_str)
				.is_some_and(|guidance| !guidance.is_empty())
		}));
	}

	#[test]
	fn search_item_marks_derived_page_snippet_with_provenance() {
		let section_id = Uuid::from_u128(20);
		let source_ref = test_source_ref(section_id);
		let row = KnowledgePageSearchRow {
			page_id: Uuid::from_u128(21),
			page_kind: "project".to_string(),
			page_key: "elf".to_string(),
			title: "ELF Knowledge".to_string(),
			status: "active".to_string(),
			source_coverage: serde_json::json!({
				"source_count": 1,
				"cited_source_count": 1,
				"coverage_complete": true
			}),
			rebuild_metadata: serde_json::json!({ "deterministic": true }),
			page_updated_at: OffsetDateTime::UNIX_EPOCH,
			rebuilt_at: OffsetDateTime::UNIX_EPOCH,
			section_id,
			section_key: "source-notes".to_string(),
			heading: "Source Notes".to_string(),
			role: "current_truth".to_string(),
			content: "Derived knowledge pages cite source notes before they are trusted."
				.to_string(),
			ordinal: 0,
			citations: serde_json::json!([{ "source_kind": "note", "source_id": source_ref.source_id }]),
			unsupported_reason: None,
			lint_error_count: 0,
			lint_warning_count: 1,
			lint_info_count: 0,
			section_source_ref_count: 1,
		};
		let item = knowledge::knowledge_page_search_item(row, vec![source_ref], "source notes");

		assert_eq!(item.result_kind, "knowledge_page_section");
		assert_eq!(item.trust_state, "derived_warning");
		assert_eq!(item.citation_count, 1);
		assert_eq!(item.source_ref_count, 1);
		assert_eq!(item.source_refs.len(), 1);
		assert!(item.derived_notice.contains("Derived knowledge page snippet"));
		assert!(item.repair_guidance.is_some());
		assert!(item.snippet.contains("source notes"));
	}

	#[test]
	fn search_source_refs_suppress_deleted_and_unreviewed_sources() {
		let section_id = Uuid::from_u128(70);
		let mut active = test_source_ref(section_id);
		let mut deleted = test_source_ref(section_id);
		let mut ignored = test_source_ref(section_id);
		let current_keys = current_source_keys_for(&[&active, &deleted, &ignored]);

		deleted.source_status = Some("deleted".to_string());
		ignored.source_status = Some("ignore".to_string());

		assert!(knowledge::recallable_source_refs(slice::from_ref(&active), &current_keys));
		assert!(!knowledge::recallable_source_refs(&[deleted], &current_keys));
		assert!(!knowledge::recallable_source_refs(&[ignored], &current_keys));

		active.source_status = None;

		assert!(!knowledge::recallable_source_refs(&[active], &current_keys));
	}

	#[test]
	fn search_source_refs_suppress_non_captured_spans() {
		let section_id = Uuid::from_u128(71);
		let mut excluded = test_source_ref(section_id);
		let mut source_ref_span = test_source_ref(section_id);
		let mut policy_span = test_source_ref(section_id);
		let mut malformed_span = test_source_ref(section_id);
		let current_keys =
			current_source_keys_for(&[&excluded, &source_ref_span, &policy_span, &malformed_span]);

		excluded.source_snapshot = serde_json::json!({
			"source_span": {
				"schema": "doc_source_span/v1",
				"status": "excluded",
				"reason_code": "WRITE_POLICY_EXCLUSION"
			}
		});
		source_ref_span.source_snapshot = serde_json::json!({
			"source_ref": {
				"source_spans": [
					{
						"schema": "doc_source_span/v1",
						"status": "redacted",
						"reason_code": "WRITE_POLICY_REDACTION"
					}
				]
			}
		});
		policy_span.source_snapshot = serde_json::json!({
			"source_ref": {
				"policy_spans": [
					{
						"schema": "doc_source_span/v1",
						"status": "excluded",
						"reason_code": "WRITE_POLICY_EXCLUSION"
					}
				]
			}
		});
		malformed_span.source_snapshot = serde_json::json!({
			"source_span": {
				"schema": "doc_source_span/v1",
				"reason_code": "WRITE_POLICY_REDACTION"
			}
		});

		assert!(!knowledge::recallable_source_refs(&[excluded], &current_keys));
		assert!(!knowledge::recallable_source_refs(&[source_ref_span], &current_keys));
		assert!(!knowledge::recallable_source_refs(&[policy_span], &current_keys));
		assert!(!knowledge::recallable_source_refs(&[malformed_span], &current_keys));
	}

	#[test]
	fn search_source_refs_suppress_nested_proposal_non_captured_spans() {
		let section_id = Uuid::from_u128(73);
		let mut proposal = test_source_ref_for(section_id, Uuid::from_u128(74), "proposal-hash");

		proposal.source_kind = KnowledgeSourceKind::Proposal.as_str().to_string();
		proposal.source_status = Some("applied".to_string());
		proposal.source_snapshot = serde_json::json!({
			"kind": "proposal",
			"proposal_id": proposal.source_id,
			"source_refs": [
				{
					"kind": "doc_chunk",
					"source_ref": {
						"policy_spans": [
							{
								"schema": "doc_source_span/v1",
								"status": "excluded",
								"reason_code": "WRITE_POLICY_EXCLUSION"
							}
						]
					}
				}
			],
			"source_snapshot": {
				"sources": [
					{
						"source_snapshot": {
							"source_span": {
								"schema": "doc_source_span/v1",
								"status": "redacted",
								"reason_code": "WRITE_POLICY_REDACTION"
							}
						}
					}
				]
			},
			"diff": {
				"after": {
					"source_ref": {
						"source_spans": [
							{
								"schema": "doc_source_span/v1",
								"status": "excluded",
								"reason_code": "WRITE_POLICY_EXCLUSION"
							}
						]
					}
				}
			}
		});

		let current_keys = current_source_keys_for(&[&proposal]);

		assert!(!knowledge::recallable_source_refs(&[proposal], &current_keys));
	}

	#[test]
	fn search_item_sanitizes_proposal_citations_and_source_refs() {
		let section_id = Uuid::from_u128(75);
		let mut source_ref = test_source_ref_for(section_id, Uuid::from_u128(76), "proposal-hash");

		source_ref.source_kind = KnowledgeSourceKind::Proposal.as_str().to_string();
		source_ref.source_status = Some("applied".to_string());
		source_ref.source_snapshot = serde_json::json!({
			"kind": "proposal",
			"proposal_id": source_ref.source_id,
			"proposal_kind": "create_derived_note",
			"source_refs": [{ "kind": "doc", "source_id": Uuid::from_u128(77) }],
			"source_snapshot": { "sources": [{ "source_snapshot": { "text": "private raw source" } }] },
			"lineage": { "parents": ["private"] },
			"diff": { "summary": "private raw diff" },
			"unsupported_claim_flags": [{ "quote": "private raw flag" }],
			"target_ref": { "text": "private raw target" }
		});

		let row = KnowledgePageSearchRow {
			page_id: Uuid::from_u128(78),
			page_kind: "project".to_string(),
			page_key: "elf".to_string(),
			title: "ELF Knowledge".to_string(),
			status: "active".to_string(),
			source_coverage: serde_json::json!({
				"source_count": 1,
				"cited_source_count": 1,
				"coverage_complete": true
			}),
			rebuild_metadata: serde_json::json!({ "deterministic": true }),
			page_updated_at: OffsetDateTime::UNIX_EPOCH,
			rebuilt_at: OffsetDateTime::UNIX_EPOCH,
			section_id,
			section_key: "reviewed-proposals".to_string(),
			heading: "Reviewed Proposals".to_string(),
			role: "proposals".to_string(),
			content: "Applied proposal create_derived_note".to_string(),
			ordinal: 0,
			citations: serde_json::json!([{
				"source_kind": "proposal",
				"source_id": source_ref.source_id,
				"source_snapshot": source_ref.source_snapshot.clone()
			}]),
			unsupported_reason: None,
			lint_error_count: 0,
			lint_warning_count: 0,
			lint_info_count: 0,
			section_source_ref_count: 1,
		};
		let item = knowledge::knowledge_page_search_item(row, vec![source_ref], "proposal");
		let citation_snapshot = &item.citations[0]["source_snapshot"];
		let source_ref_snapshot = &item.source_refs[0].source_snapshot;

		assert_eq!(citation_snapshot["sanitized"], true);
		assert_eq!(source_ref_snapshot["sanitized"], true);
		assert!(citation_snapshot.get("source_refs").is_none());
		assert!(citation_snapshot.get("source_snapshot").is_none());
		assert!(citation_snapshot.get("diff").is_none());
		assert!(source_ref_snapshot.get("source_refs").is_none());
		assert!(source_ref_snapshot.get("source_snapshot").is_none());
		assert!(source_ref_snapshot.get("diff").is_none());
	}

	#[test]
	fn search_source_refs_suppress_missing_current_sources() {
		let section_id = Uuid::from_u128(72);
		let source_ref = test_source_ref(section_id);

		assert!(!knowledge::recallable_source_refs(&[source_ref], &BTreeSet::new()));
	}

	#[test]
	fn source_row_read_allowed_requires_shared_grant_for_other_agent_sources() {
		let allowed_scopes = vec!["agent_private".to_string(), "project_shared".to_string()];
		let shared_grants = HashSet::new();

		assert!(knowledge::source_row_read_allowed(
			"owner-agent",
			"project_shared",
			Some("owner-agent"),
			&allowed_scopes,
			&shared_grants
		));
		assert!(!knowledge::source_row_read_allowed(
			"owner-agent",
			"project_shared",
			Some("reader-agent"),
			&allowed_scopes,
			&shared_grants
		));

		let shared_grants = HashSet::from([SharedSpaceGrantKey {
			scope: "project_shared".to_string(),
			space_owner_agent_id: "owner-agent".to_string(),
		}]);

		assert!(knowledge::source_row_read_allowed(
			"owner-agent",
			"project_shared",
			Some("reader-agent"),
			&allowed_scopes,
			&shared_grants
		));
	}

	fn test_page() -> KnowledgePage {
		KnowledgePage {
			page_id: Uuid::from_u128(1),
			tenant_id: "tenant".to_string(),
			project_id: "project".to_string(),
			page_kind: "project".to_string(),
			page_key: "elf".to_string(),
			title: "ELF".to_string(),
			contract_schema: "elf.knowledge_page/v1".to_string(),
			status: "active".to_string(),
			rebuild_source_hash: "source-hash".to_string(),
			content_hash: "content-hash".to_string(),
			source_coverage: serde_json::json!({
				"source_count": 2,
				"cited_source_count": 1,
				"coverage_complete": false
			}),
			source_snapshot: serde_json::json!({}),
			rebuild_metadata: serde_json::json!({}),
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			rebuilt_at: OffsetDateTime::UNIX_EPOCH,
		}
	}

	fn test_section(
		section_id: Uuid,
		section_key: &str,
		citations: serde_json::Value,
		unsupported_reason: Option<String>,
	) -> KnowledgePageSection {
		KnowledgePageSection {
			section_id,
			page_id: Uuid::from_u128(1),
			section_key: section_key.to_string(),
			heading: section_key.to_string(),
			role: "current_truth".to_string(),
			content: "Section content.".to_string(),
			ordinal: 0,
			citations,
			unsupported_reason,
			content_hash: "section-hash".to_string(),
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
		}
	}

	fn test_source_ref(section_id: Uuid) -> KnowledgePageSourceRef {
		test_source_ref_for(section_id, Uuid::from_u128(31), "source-hash")
	}

	fn test_source_ref_for(
		section_id: Uuid,
		source_id: Uuid,
		source_hash: &str,
	) -> KnowledgePageSourceRef {
		KnowledgePageSourceRef {
			ref_id: Uuid::from_u128(30),
			page_id: Uuid::from_u128(21),
			section_id: Some(section_id),
			source_kind: "note".to_string(),
			source_id,
			source_status: Some("active".to_string()),
			source_updated_at: Some(OffsetDateTime::UNIX_EPOCH),
			source_content_hash: Some(source_hash.to_string()),
			source_snapshot: serde_json::json!({
				"schema": "test_source/v1",
				"source_id": source_id,
				"content_hash": source_hash,
			}),
			citation_metadata: serde_json::json!({}),
			created_at: OffsetDateTime::UNIX_EPOCH,
		}
	}

	fn current_source_keys_for(source_refs: &[&KnowledgePageSourceRef]) -> BTreeSet<String> {
		source_refs
			.iter()
			.map(|source_ref| {
				knowledge::current_key(source_ref.source_kind.as_str(), source_ref.source_id)
			})
			.collect()
	}

	fn test_page_response(section_id: Uuid, source_id: Uuid) -> KnowledgePageResponse {
		let page = test_page();
		let section = test_section(
			section_id,
			"source-notes",
			serde_json::json!([{ "source_kind": "note", "source_id": source_id }]),
			None,
		);
		let source_ref = test_source_ref_for(section_id, source_id, "new-hash");

		KnowledgePageResponse {
			page: KnowledgePageSummary::from(page),
			sections: vec![KnowledgePageSectionResponse {
				citation_count: 1,
				source_ref_count: 1,
				coverage_complete: true,
				source_backlinks: Vec::new(),
				..KnowledgePageSectionResponse::from(section)
			}],
			source_refs: vec![KnowledgePageSourceRefResponse::from(source_ref)],
			lint_findings: Vec::new(),
		}
	}

	fn assert_candidate_is_reviewable(candidate: &KnowledgeDeltaMemoryCandidate) {
		assert_eq!(candidate.reason, "changed_claim");
		assert_eq!(candidate.source_refs.len(), 1);
		assert_eq!(candidate.source_refs[0].kind.as_str(), "note");
		assert_eq!(candidate.source_snapshot["source_mutation_allowed"], false);
		assert_eq!(candidate.diff.after["reason"], "changed_claim");
		assert_eq!(candidate.proposed_payload["type"], "plan");
		assert_eq!(candidate.proposed_payload["source_ref"]["schema"], "elf.knowledge_delta/v1");
	}
}
