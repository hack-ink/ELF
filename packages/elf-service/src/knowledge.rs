//! Deterministic derived knowledge page rebuild and readback service APIs.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::{self, Map, Value};
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ElfService, Error, Result};
use elf_domain::{
	english_gate,
	knowledge::{
		KNOWLEDGE_PAGE_CONTRACT_SCHEMA_V1, KNOWLEDGE_PAGE_REBUILD_SCHEMA_V1,
		KNOWLEDGE_PAGE_SOURCE_COVERAGE_SCHEMA_V1, KNOWLEDGE_PAGE_VERSION_DIFF_SCHEMA_V1,
		KnowledgePageKind, KnowledgeSourceKind,
	},
};
use elf_storage::{
	knowledge::{
		self, KnowledgeDocChunkSource, KnowledgeDocSource, KnowledgeEventSource,
		KnowledgeNoteSource, KnowledgePageLintFindingInsert, KnowledgePageSearchRow,
		KnowledgePageSectionInsert, KnowledgePageSourceRefInsert, KnowledgePageUpsert,
		KnowledgeProposalSource, KnowledgeRelationSource,
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
	/// English-only query for page title, key, heading, or section content.
	pub query: String,
	/// Optional page-kind filter.
	pub page_kind: Option<KnowledgePageKind>,
	/// Maximum number of section snippets to return.
	pub limit: Option<u32>,
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
					"all requested knowledge page sources must exist, document sources must be active, and proposals must be applied"
						.to_string(),
			});
		}

		Ok(())
	}
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
		validate_non_empty("query", req.query.as_str())?;

		if !english_gate::is_english_natural_language(req.query.as_str()) {
			return Err(Error::NonEnglishInput { field: "$.query".to_string() });
		}

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
		let source_refs_by_section = source_refs_by_section(&source_refs);
		let items = rows
			.into_iter()
			.map(|row| {
				let refs = cloned_source_refs(source_refs_by_section.get(&row.section_id));

				knowledge_page_search_item(row, refs, req.query.as_str())
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
		let (docs, doc_chunks, notes, events, relations, proposals) = self
			.resolve_existing_source_rows(req.tenant_id.as_str(), req.project_id.as_str(), ids)
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
			&ids.doc_ids,
		)
		.await?;
		let doc_chunks = knowledge::fetch_knowledge_doc_chunk_sources(
			&self.db.pool,
			tenant_id,
			project_id,
			&ids.doc_chunk_ids,
		)
		.await?;
		let notes = knowledge::fetch_knowledge_note_sources(
			&self.db.pool,
			tenant_id,
			project_id,
			&ids.note_ids,
		)
		.await?;
		let events = knowledge::fetch_knowledge_event_sources(
			&self.db.pool,
			tenant_id,
			project_id,
			&ids.event_ids,
		)
		.await?;
		let relations = knowledge::fetch_knowledge_relation_sources(
			&self.db.pool,
			tenant_id,
			project_id,
			&ids.relation_ids,
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
			.resolve_existing_source_rows(page.tenant_id.as_str(), page.project_id.as_str(), ids)
			.await?;
		let mut sources = source_snapshots(docs, doc_chunks, notes, events, relations, proposals);

		Ok(sources.drain(..).map(|source| (source_key(&source), source)).collect())
	}
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
		citations: row.citations,
		citation_count,
		source_ref_count,
		source_refs: source_refs.into_iter().map(KnowledgePageSourceRefResponse::from).collect(),
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
	let summary =
		row.diff.get("summary").and_then(Value::as_str).unwrap_or("Applied consolidation proposal");
	let line = format!("Applied proposal {}: {summary}", row.proposal_kind);
	let snapshot = serde_json::json!({
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
	});

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
	use crate::knowledge::{
		self, DraftSection, KnowledgePage, KnowledgePageKind, KnowledgePageSearchRow,
		KnowledgePageSection, KnowledgePageSourceRef, KnowledgeSourceKind, OffsetDateTime,
		SourceSnapshot, Uuid,
	};

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
		KnowledgePageSourceRef {
			ref_id: Uuid::from_u128(30),
			page_id: Uuid::from_u128(21),
			section_id: Some(section_id),
			source_kind: "note".to_string(),
			source_id: Uuid::from_u128(31),
			source_status: Some("active".to_string()),
			source_updated_at: Some(OffsetDateTime::UNIX_EPOCH),
			source_content_hash: Some("source-hash".to_string()),
			source_snapshot: serde_json::json!({}),
			citation_metadata: serde_json::json!({}),
			created_at: OffsetDateTime::UNIX_EPOCH,
		}
	}
}
