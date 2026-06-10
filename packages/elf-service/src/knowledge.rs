//! Deterministic derived knowledge page rebuild and readback service APIs.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{self, Map, Value};
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ElfService, Error, Result};
use elf_domain::knowledge::{
	KNOWLEDGE_PAGE_CONTRACT_SCHEMA_V1, KNOWLEDGE_PAGE_REBUILD_SCHEMA_V1,
	KNOWLEDGE_PAGE_SOURCE_COVERAGE_SCHEMA_V1, KnowledgePageKind, KnowledgeSourceKind,
};
use elf_storage::{
	knowledge::{
		self, KnowledgeEventSource, KnowledgeNoteSource, KnowledgePageLintFindingInsert,
		KnowledgePageSectionInsert, KnowledgePageSourceRefInsert, KnowledgePageUpsert,
		KnowledgeProposalSource, KnowledgeRelationSource,
	},
	models::{
		KnowledgePage, KnowledgePageLintFinding, KnowledgePageSection, KnowledgePageSourceRef,
	},
};

const DEFAULT_LIST_LIMIT: i64 = 50;
const MAX_LIST_LIMIT: i64 = 200;

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

/// Response returned after linting one knowledge page.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageLintResponse {
	/// Page identifier.
	pub page_id: Uuid,
	/// Current lint findings.
	pub findings: Vec<KnowledgePageLintFindingResponse>,
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
			content_hash: section.content_hash,
			created_at: section.created_at,
			updated_at: section.updated_at,
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
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}
impl From<KnowledgePageLintFinding> for KnowledgePageLintFindingResponse {
	fn from(finding: KnowledgePageLintFinding) -> Self {
		Self {
			finding_id: finding.finding_id,
			page_id: finding.page_id,
			section_id: finding.section_id,
			finding_type: finding.finding_type,
			severity: finding.severity,
			source_kind: finding.source_kind,
			source_id: finding.source_id,
			message: finding.message,
			details: finding.details,
			created_at: finding.created_at,
		}
	}
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
	note_ids: Vec<Uuid>,
	event_ids: Vec<Uuid>,
	relation_ids: Vec<Uuid>,
	proposal_ids: Vec<Uuid>,
}
impl SourceIds {
	fn from_request(req: &KnowledgePageRebuildRequest) -> Result<Self> {
		let ids = Self {
			note_ids: sorted_unique(&req.note_ids),
			event_ids: sorted_unique(&req.event_ids),
			relation_ids: sorted_unique(&req.relation_ids),
			proposal_ids: sorted_unique(&req.proposal_ids),
		};

		ids.validate_non_empty()?;

		Ok(ids)
	}

	fn from_source_refs(source_refs: &[KnowledgePageSourceRef]) -> Result<Self> {
		let mut note_ids = Vec::new();
		let mut event_ids = Vec::new();
		let mut relation_ids = Vec::new();
		let mut proposal_ids = Vec::new();

		for source_ref in source_refs {
			match KnowledgeSourceKind::parse(source_ref.source_kind.as_str()) {
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
			note_ids: sorted_unique(&note_ids),
			event_ids: sorted_unique(&event_ids),
			relation_ids: sorted_unique(&relation_ids),
			proposal_ids: sorted_unique(&proposal_ids),
		})
	}

	fn validate_non_empty(&self) -> Result<()> {
		if self.note_ids.is_empty()
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
		notes: usize,
		events: usize,
		relations: usize,
		proposals: usize,
	) -> Result<()> {
		if notes != self.note_ids.len()
			|| events != self.event_ids.len()
			|| relations != self.relation_ids.len()
			|| proposals != self.proposal_ids.len()
		{
			return Err(Error::InvalidRequest {
				message:
					"all requested knowledge page sources must exist and proposals must be applied"
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
		let rebuild_metadata = rebuild_metadata(&source_hash, &req.provider_metadata);
		let content_hash =
			page_content_hash(&title, &sections, &source_coverage, &rebuild_metadata)?;
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
		let findings = self.lint_source_refs(&page, &source_refs).await?;
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
		let sections = knowledge::list_knowledge_page_sections(&self.db.pool, page_id)
			.await?
			.into_iter()
			.map(KnowledgePageSectionResponse::from)
			.collect();
		let source_refs = knowledge::list_knowledge_page_source_refs(&self.db.pool, page_id)
			.await?
			.into_iter()
			.map(KnowledgePageSourceRefResponse::from)
			.collect();
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
		let notes = knowledge::fetch_knowledge_note_sources(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			&ids.note_ids,
		)
		.await?;
		let events = knowledge::fetch_knowledge_event_sources(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			&ids.event_ids,
		)
		.await?;
		let relations = knowledge::fetch_knowledge_relation_sources(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			&ids.relation_ids,
		)
		.await?;
		let proposals = knowledge::fetch_knowledge_proposal_sources(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			&ids.proposal_ids,
		)
		.await?;

		ids.require_counts(notes.len(), events.len(), relations.len(), proposals.len())?;

		let mut sources = Vec::new();

		sources.extend(notes.into_iter().map(note_source_snapshot));
		sources.extend(events.into_iter().map(event_source_snapshot));
		sources.extend(relations.into_iter().map(relation_source_snapshot));
		sources.extend(proposals.into_iter().map(proposal_source_snapshot));
		sources.sort_by_key(source_sort_key);

		Ok(sources)
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
		let req = KnowledgePageRebuildRequest {
			tenant_id: page.tenant_id.clone(),
			project_id: page.project_id.clone(),
			agent_id: String::new(),
			page_kind: KnowledgePageKind::parse(page.page_kind.as_str()).ok_or_else(|| {
				Error::InvalidRequest {
					message: "stored knowledge page kind is invalid".to_string(),
				}
			})?,
			page_key: page.page_key.clone(),
			title: Some(page.title.clone()),
			note_ids: ids.note_ids.clone(),
			event_ids: ids.event_ids.clone(),
			relation_ids: ids.relation_ids.clone(),
			proposal_ids: ids.proposal_ids.clone(),
			provider_metadata: empty_object(),
		};
		let mut sources = self.resolve_sources(&req, ids).await?;

		Ok(sources.drain(..).map(|source| (source_key(&source), source)).collect())
	}
}

fn build_sections(sources: &[SourceSnapshot]) -> Result<Vec<DraftSection>> {
	let note_indexes = source_indexes(sources, KnowledgeSourceKind::Note);
	let event_indexes = source_indexes(sources, KnowledgeSourceKind::Event);
	let relation_indexes = source_indexes(sources, KnowledgeSourceKind::Relation);
	let proposal_indexes = source_indexes(sources, KnowledgeSourceKind::Proposal);
	let mut sections = Vec::new();

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
				finding_type: "unsupported_section".to_string(),
				severity: "warning".to_string(),
				source_kind: None,
				source_id: None,
				message: format!("Knowledge page section lacks citations: {reason}"),
				details: serde_json::json!({ "section_key": section.section_key }),
			})
		})
		.collect()
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

fn rebuild_metadata(source_hash: &str, provider_metadata: &Value) -> Value {
	let llm_derived =
		provider_metadata.get("llm_derived").and_then(Value::as_bool).unwrap_or(false);

	serde_json::json!({
		"schema": KNOWLEDGE_PAGE_REBUILD_SCHEMA_V1,
		"source_snapshot_hash": source_hash,
		"deterministic": !llm_derived,
		"provider_metadata": provider_metadata,
		"allowed_variance": if llm_derived {
			serde_json::json!(["LLM-derived page text may vary; provider metadata records the nondeterministic input path."])
		} else {
			serde_json::json!([])
		},
	})
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
	hash_json(&serde_json::json!({
		"title": title,
		"sections": sections.iter().map(section_hash_payload).collect::<Vec<_>>(),
		"source_coverage": source_coverage,
		"rebuild_metadata": rebuild_metadata,
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
		}),
	}
}

fn source_changed(source_ref: &KnowledgePageSourceRef, current: &SourceSnapshot) -> bool {
	source_ref.source_status.as_deref() != current.status.as_deref()
		|| source_ref.source_updated_at != current.updated_at
		|| source_ref.source_content_hash.as_deref() != current.content_hash.as_deref()
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
		self, KnowledgePageKind, KnowledgePageSourceRef, KnowledgeSourceKind, OffsetDateTime,
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

	#[test]
	fn build_sections_preserves_citations_and_deterministic_hashes() {
		let sources = vec![
			test_source(KnowledgeSourceKind::Note, 1, "A source note supports the page."),
			test_source(KnowledgeSourceKind::Event, 2, "An event audit supports the page."),
			test_source(KnowledgeSourceKind::Relation, 3, "A relation supports the page."),
			test_source(KnowledgeSourceKind::Proposal, 4, "An applied proposal supports the page."),
		];
		let mut first_sections =
			knowledge::build_sections(&sources).expect("sections should build");

		for section in &mut first_sections {
			section.citations = knowledge::citations_value(section, &sources);
			section.content_hash = knowledge::hash_json(&knowledge::section_hash_payload(section))
				.expect("section hash should serialize");
		}

		assert_eq!(first_sections.len(), 4);
		assert!(first_sections.iter().all(|section| {
			section.citations.as_array().is_some_and(|citations| !citations.is_empty())
		}));

		let coverage = knowledge::source_coverage_value(
			KnowledgePageKind::Project,
			"elf",
			&first_sections,
			&sources,
		);
		let metadata = knowledge::rebuild_metadata("source-hash", &knowledge::empty_object());
		let first_hash = knowledge::page_content_hash("ELF", &first_sections, &coverage, &metadata)
			.expect("page hash should serialize");
		let second_hash =
			knowledge::page_content_hash("ELF", &first_sections, &coverage, &metadata)
				.expect("page hash should serialize");

		assert_eq!(coverage["coverage_complete"], true);
		assert_eq!(metadata["deterministic"], true);
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
		);

		assert_eq!(metadata["deterministic"], false);
		assert!(metadata["allowed_variance"].as_array().is_some_and(|items| !items.is_empty()));
		assert_eq!(metadata["provider_metadata"]["provider_id"], "fixture");
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
}
