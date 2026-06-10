//! Derived knowledge page persistence and source-snapshot queries.

use serde_json::Value;
use sqlx::{FromRow, PgExecutor};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	Result,
	models::{
		KnowledgePage, KnowledgePageLintFinding, KnowledgePageSection, KnowledgePageSourceRef,
	},
};

/// Arguments for upserting one derived knowledge page.
pub struct KnowledgePageUpsert<'a> {
	/// Page identifier to use for a newly created page.
	pub page_id: Uuid,
	/// Tenant that owns the page.
	pub tenant_id: &'a str,
	/// Project that owns the page.
	pub project_id: &'a str,
	/// Page kind.
	pub page_kind: &'a str,
	/// Stable page key.
	pub page_key: &'a str,
	/// Page title.
	pub title: &'a str,
	/// Versioned page contract schema.
	pub contract_schema: &'a str,
	/// Page lifecycle status.
	pub status: &'a str,
	/// Canonical source snapshot hash.
	pub rebuild_source_hash: &'a str,
	/// Canonical page content hash.
	pub content_hash: &'a str,
	/// Source coverage metadata.
	pub source_coverage: &'a Value,
	/// Aggregate source snapshot metadata.
	pub source_snapshot: &'a Value,
	/// Rebuild metadata.
	pub rebuild_metadata: &'a Value,
	/// Rebuild timestamp.
	pub now: OffsetDateTime,
}

/// Arguments for inserting one knowledge page section.
pub struct KnowledgePageSectionInsert<'a> {
	/// Section identifier.
	pub section_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Stable section key.
	pub section_key: &'a str,
	/// Section heading.
	pub heading: &'a str,
	/// Section role.
	pub role: &'a str,
	/// Section content.
	pub content: &'a str,
	/// Section display order.
	pub ordinal: i32,
	/// Section citations.
	pub citations: &'a Value,
	/// Reason the section has no citations, when intentionally unsupported.
	pub unsupported_reason: Option<&'a str>,
	/// Section content hash.
	pub content_hash: &'a str,
	/// Creation/update timestamp.
	pub now: OffsetDateTime,
}

/// Arguments for inserting one normalized knowledge page citation.
pub struct KnowledgePageSourceRefInsert<'a> {
	/// Source-reference row identifier.
	pub ref_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Section that cites the source, if section-scoped.
	pub section_id: Option<Uuid>,
	/// Source kind.
	pub source_kind: &'a str,
	/// Authoritative source identifier.
	pub source_id: Uuid,
	/// Captured source status.
	pub source_status: Option<&'a str>,
	/// Captured source updated timestamp.
	pub source_updated_at: Option<OffsetDateTime>,
	/// Captured source content hash.
	pub source_content_hash: Option<&'a str>,
	/// Captured source snapshot.
	pub source_snapshot: &'a Value,
	/// Citation-local metadata.
	pub citation_metadata: &'a Value,
	/// Creation timestamp.
	pub now: OffsetDateTime,
}

/// Arguments for inserting one knowledge page lint finding.
pub struct KnowledgePageLintFindingInsert<'a> {
	/// Lint finding identifier.
	pub finding_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Section associated with the finding, when available.
	pub section_id: Option<Uuid>,
	/// Finding type.
	pub finding_type: &'a str,
	/// Finding severity.
	pub severity: &'a str,
	/// Source kind associated with the finding, when available.
	pub source_kind: Option<&'a str>,
	/// Source identifier associated with the finding, when available.
	pub source_id: Option<Uuid>,
	/// Human-readable finding message.
	pub message: &'a str,
	/// Structured finding details.
	pub details: &'a Value,
	/// Creation timestamp.
	pub now: OffsetDateTime,
}

/// Authoritative note source row used by the knowledge page rebuilder.
#[derive(Debug, FromRow)]
pub struct KnowledgeNoteSource {
	/// Note identifier.
	pub note_id: Uuid,
	/// Agent that owns the note.
	pub agent_id: String,
	/// Note scope.
	pub scope: String,
	/// Note type.
	pub note_type: String,
	/// Optional note key.
	pub key: Option<String>,
	/// Note text.
	pub text: String,
	/// Note importance.
	pub importance: f32,
	/// Note confidence.
	pub confidence: f32,
	/// Note status.
	pub status: String,
	/// Note creation timestamp.
	pub created_at: OffsetDateTime,
	/// Note update timestamp.
	pub updated_at: OffsetDateTime,
	/// Optional note expiry timestamp.
	pub expires_at: Option<OffsetDateTime>,
	/// Note embedding version.
	pub embedding_version: String,
	/// Opaque note source reference.
	pub source_ref: Value,
}

/// Durable add_event audit source row used by the knowledge page rebuilder.
#[derive(Debug, FromRow)]
pub struct KnowledgeEventSource {
	/// Ingest decision identifier.
	pub decision_id: Uuid,
	/// Agent that wrote the audited event-derived note decision.
	pub agent_id: String,
	/// Scope associated with the audited decision.
	pub scope: String,
	/// Ingestion pipeline name.
	pub pipeline: String,
	/// Event-derived note type.
	pub note_type: String,
	/// Optional note key.
	pub note_key: Option<String>,
	/// Note identifier affected by the decision, when persisted.
	pub note_id: Option<Uuid>,
	/// Policy decision.
	pub policy_decision: String,
	/// Note operation.
	pub note_op: String,
	/// Optional reason code.
	pub reason_code: Option<String>,
	/// Structured audit details.
	pub details: Value,
	/// Audit timestamp.
	pub ts: OffsetDateTime,
}

/// Authoritative graph relation source row used by the knowledge page rebuilder.
#[derive(Debug, FromRow)]
pub struct KnowledgeRelationSource {
	/// Graph fact identifier.
	pub fact_id: Uuid,
	/// Agent that wrote the fact.
	pub agent_id: String,
	/// Fact scope.
	pub scope: String,
	/// Subject canonical text.
	pub subject: String,
	/// Optional subject kind.
	pub subject_kind: Option<String>,
	/// Predicate text.
	pub predicate: String,
	/// Optional object entity canonical text.
	pub object_entity: Option<String>,
	/// Optional object entity kind.
	pub object_kind: Option<String>,
	/// Optional scalar object value.
	pub object_value: Option<String>,
	/// Fact validity window start.
	pub valid_from: OffsetDateTime,
	/// Fact validity window end, when historical.
	pub valid_to: Option<OffsetDateTime>,
	/// Fact update timestamp.
	pub updated_at: OffsetDateTime,
	/// Evidence notes linked to this fact.
	pub evidence_notes: Value,
}

/// Reviewed consolidation proposal source row used by the knowledge page rebuilder.
#[derive(Debug, FromRow)]
pub struct KnowledgeProposalSource {
	/// Consolidation proposal identifier.
	pub proposal_id: Uuid,
	/// Parent consolidation run identifier.
	pub run_id: Uuid,
	/// Agent that registered the proposal.
	pub agent_id: String,
	/// Proposal kind.
	pub proposal_kind: String,
	/// Proposal apply intent.
	pub apply_intent: String,
	/// Proposal review state.
	pub review_state: String,
	/// Serialized proposal source references.
	pub source_refs: Value,
	/// Serialized proposal source snapshot.
	pub source_snapshot: Value,
	/// Serialized proposal lineage.
	pub lineage: Value,
	/// Serialized proposal diff.
	pub diff: Value,
	/// Proposal confidence.
	pub confidence: f32,
	/// Unsupported claim flags.
	pub unsupported_claim_flags: Value,
	/// Contradiction markers.
	pub contradiction_markers: Value,
	/// Staleness markers.
	pub staleness_markers: Value,
	/// Derived target reference.
	pub target_ref: Value,
	/// Proposed derived payload.
	pub proposed_payload: Value,
	/// Proposal update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Upserts one derived knowledge page and returns the persisted row.
pub async fn upsert_knowledge_page<'e, E>(
	executor: E,
	args: KnowledgePageUpsert<'_>,
) -> Result<KnowledgePage>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, KnowledgePage>(
		"\
INSERT INTO knowledge_pages (
	page_id,
	tenant_id,
	project_id,
	page_kind,
	page_key,
	title,
	contract_schema,
	status,
	rebuild_source_hash,
	content_hash,
	source_coverage,
	source_snapshot,
	rebuild_metadata,
	created_at,
	updated_at,
	rebuilt_at
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$14,$14)
ON CONFLICT (tenant_id, project_id, page_kind, page_key) DO UPDATE
SET
	title = EXCLUDED.title,
	contract_schema = EXCLUDED.contract_schema,
	status = EXCLUDED.status,
	rebuild_source_hash = EXCLUDED.rebuild_source_hash,
	content_hash = EXCLUDED.content_hash,
	source_coverage = EXCLUDED.source_coverage,
	source_snapshot = EXCLUDED.source_snapshot,
	rebuild_metadata = EXCLUDED.rebuild_metadata,
	updated_at = EXCLUDED.updated_at,
	rebuilt_at = EXCLUDED.rebuilt_at
RETURNING
	page_id,
	tenant_id,
	project_id,
	page_kind,
	page_key,
	title,
	contract_schema,
	status,
	rebuild_source_hash,
	content_hash,
	source_coverage,
	source_snapshot,
	rebuild_metadata,
	created_at,
	updated_at,
	rebuilt_at",
	)
	.bind(args.page_id)
	.bind(args.tenant_id)
	.bind(args.project_id)
	.bind(args.page_kind)
	.bind(args.page_key)
	.bind(args.title)
	.bind(args.contract_schema)
	.bind(args.status)
	.bind(args.rebuild_source_hash)
	.bind(args.content_hash)
	.bind(args.source_coverage)
	.bind(args.source_snapshot)
	.bind(args.rebuild_metadata)
	.bind(args.now)
	.fetch_one(executor)
	.await?;

	Ok(row)
}

/// Deletes all section, citation, and lint child rows for a page before rebuild.
pub async fn delete_knowledge_page_children<'e, E>(executor: E, page_id: Uuid) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
DELETE FROM knowledge_page_lint_findings WHERE page_id = $1;
DELETE FROM knowledge_page_source_refs WHERE page_id = $1;
DELETE FROM knowledge_page_sections WHERE page_id = $1;",
	)
	.bind(page_id)
	.execute(executor)
	.await?;

	Ok(())
}

/// Inserts one derived knowledge page section.
pub async fn insert_knowledge_page_section<'e, E>(
	executor: E,
	args: KnowledgePageSectionInsert<'_>,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO knowledge_page_sections (
	section_id,
	page_id,
	section_key,
	heading,
	role,
	content,
	ordinal,
	citations,
	unsupported_reason,
	content_hash,
	created_at,
	updated_at
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$11)",
	)
	.bind(args.section_id)
	.bind(args.page_id)
	.bind(args.section_key)
	.bind(args.heading)
	.bind(args.role)
	.bind(args.content)
	.bind(args.ordinal)
	.bind(args.citations)
	.bind(args.unsupported_reason)
	.bind(args.content_hash)
	.bind(args.now)
	.execute(executor)
	.await?;

	Ok(())
}

/// Inserts one normalized knowledge page citation/source reference.
pub async fn insert_knowledge_page_source_ref<'e, E>(
	executor: E,
	args: KnowledgePageSourceRefInsert<'_>,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO knowledge_page_source_refs (
	ref_id,
	page_id,
	section_id,
	source_kind,
	source_id,
	source_status,
	source_updated_at,
	source_content_hash,
	source_snapshot,
	citation_metadata,
	created_at
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
	)
	.bind(args.ref_id)
	.bind(args.page_id)
	.bind(args.section_id)
	.bind(args.source_kind)
	.bind(args.source_id)
	.bind(args.source_status)
	.bind(args.source_updated_at)
	.bind(args.source_content_hash)
	.bind(args.source_snapshot)
	.bind(args.citation_metadata)
	.bind(args.now)
	.execute(executor)
	.await?;

	Ok(())
}

/// Inserts one knowledge page lint finding.
pub async fn insert_knowledge_page_lint_finding<'e, E>(
	executor: E,
	args: KnowledgePageLintFindingInsert<'_>,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO knowledge_page_lint_findings (
	finding_id,
	page_id,
	section_id,
	finding_type,
	severity,
	source_kind,
	source_id,
	message,
	details,
	created_at
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
	)
	.bind(args.finding_id)
	.bind(args.page_id)
	.bind(args.section_id)
	.bind(args.finding_type)
	.bind(args.severity)
	.bind(args.source_kind)
	.bind(args.source_id)
	.bind(args.message)
	.bind(args.details)
	.bind(args.now)
	.execute(executor)
	.await?;

	Ok(())
}

/// Deletes persisted lint findings for one page.
pub async fn delete_knowledge_page_lint_findings<'e, E>(executor: E, page_id: Uuid) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query("DELETE FROM knowledge_page_lint_findings WHERE page_id = $1")
		.bind(page_id)
		.execute(executor)
		.await?;

	Ok(())
}

/// Fetches one knowledge page by identifier.
pub async fn get_knowledge_page<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	page_id: Uuid,
) -> Result<Option<KnowledgePage>>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, KnowledgePage>(
		"\
SELECT
	page_id,
	tenant_id,
	project_id,
	page_kind,
	page_key,
	title,
	contract_schema,
	status,
	rebuild_source_hash,
	content_hash,
	source_coverage,
	source_snapshot,
	rebuild_metadata,
	created_at,
	updated_at,
	rebuilt_at
FROM knowledge_pages
WHERE tenant_id = $1 AND project_id = $2 AND page_id = $3
LIMIT 1",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(page_id)
	.fetch_optional(executor)
	.await?;

	Ok(row)
}

/// Lists knowledge pages for a tenant and project.
pub async fn list_knowledge_pages<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	page_kind: Option<&str>,
	limit: i64,
) -> Result<Vec<KnowledgePage>>
where
	E: PgExecutor<'e>,
{
	let rows = sqlx::query_as::<_, KnowledgePage>(
		"\
SELECT
	page_id,
	tenant_id,
	project_id,
	page_kind,
	page_key,
	title,
	contract_schema,
	status,
	rebuild_source_hash,
	content_hash,
	source_coverage,
	source_snapshot,
	rebuild_metadata,
	created_at,
	updated_at,
	rebuilt_at
FROM knowledge_pages
WHERE tenant_id = $1
	AND project_id = $2
	AND ($3::text IS NULL OR page_kind = $3)
ORDER BY updated_at DESC, page_id DESC
LIMIT $4",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(page_kind)
	.bind(limit)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}

/// Lists sections for one knowledge page.
pub async fn list_knowledge_page_sections<'e, E>(
	executor: E,
	page_id: Uuid,
) -> Result<Vec<KnowledgePageSection>>
where
	E: PgExecutor<'e>,
{
	let rows = sqlx::query_as::<_, KnowledgePageSection>(
		"\
SELECT
	section_id,
	page_id,
	section_key,
	heading,
	role,
	content,
	ordinal,
	citations,
	unsupported_reason,
	content_hash,
	created_at,
	updated_at
FROM knowledge_page_sections
WHERE page_id = $1
ORDER BY ordinal ASC, section_key ASC",
	)
	.bind(page_id)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}

/// Lists normalized source refs for one knowledge page.
pub async fn list_knowledge_page_source_refs<'e, E>(
	executor: E,
	page_id: Uuid,
) -> Result<Vec<KnowledgePageSourceRef>>
where
	E: PgExecutor<'e>,
{
	let rows = sqlx::query_as::<_, KnowledgePageSourceRef>(
		"\
SELECT
	ref_id,
	page_id,
	section_id,
	source_kind,
	source_id,
	source_status,
	source_updated_at,
	source_content_hash,
	source_snapshot,
	citation_metadata,
	created_at
FROM knowledge_page_source_refs
WHERE page_id = $1
ORDER BY source_kind ASC, source_id ASC, ref_id ASC",
	)
	.bind(page_id)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}

/// Lists lint findings for one knowledge page.
pub async fn list_knowledge_page_lint_findings<'e, E>(
	executor: E,
	page_id: Uuid,
) -> Result<Vec<KnowledgePageLintFinding>>
where
	E: PgExecutor<'e>,
{
	let rows = sqlx::query_as::<_, KnowledgePageLintFinding>(
		"\
SELECT
	finding_id,
	page_id,
	section_id,
	finding_type,
	severity,
	source_kind,
	source_id,
	message,
	details,
	created_at
FROM knowledge_page_lint_findings
WHERE page_id = $1
ORDER BY severity DESC, created_at ASC, finding_id ASC",
	)
	.bind(page_id)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}

/// Fetches note sources by identifier for a knowledge page rebuild.
pub async fn fetch_knowledge_note_sources<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	note_ids: &[Uuid],
) -> Result<Vec<KnowledgeNoteSource>>
where
	E: PgExecutor<'e>,
{
	if note_ids.is_empty() {
		return Ok(Vec::new());
	}

	let rows = sqlx::query_as::<_, KnowledgeNoteSource>(
		"\
SELECT
	note_id,
	agent_id,
	scope,
	type AS note_type,
	key,
	text,
	importance,
	confidence,
	status,
	created_at,
	updated_at,
	expires_at,
	embedding_version,
	source_ref
FROM memory_notes
WHERE tenant_id = $1
	AND project_id = $2
	AND note_id = ANY($3::uuid[])
ORDER BY updated_at ASC, note_id ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(note_ids)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}

/// Fetches durable add_event audit sources by decision identifier.
pub async fn fetch_knowledge_event_sources<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	decision_ids: &[Uuid],
) -> Result<Vec<KnowledgeEventSource>>
where
	E: PgExecutor<'e>,
{
	if decision_ids.is_empty() {
		return Ok(Vec::new());
	}

	let rows = sqlx::query_as::<_, KnowledgeEventSource>(
		"\
SELECT
	decision_id,
	agent_id,
	scope,
	pipeline,
	note_type,
	note_key,
	note_id,
	policy_decision,
	note_op,
	reason_code,
	details,
	ts
FROM memory_ingest_decisions
WHERE tenant_id = $1
	AND project_id = $2
	AND decision_id = ANY($3::uuid[])
	AND pipeline = 'add_event'
ORDER BY ts ASC, decision_id ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(decision_ids)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}

/// Fetches relation sources by graph fact identifier for a knowledge page rebuild.
pub async fn fetch_knowledge_relation_sources<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	fact_ids: &[Uuid],
) -> Result<Vec<KnowledgeRelationSource>>
where
	E: PgExecutor<'e>,
{
	if fact_ids.is_empty() {
		return Ok(Vec::new());
	}

	let rows = sqlx::query_as::<_, KnowledgeRelationSource>(
		"\
SELECT
	gf.fact_id,
	gf.agent_id,
	gf.scope,
	subject.canonical AS subject,
	subject.kind AS subject_kind,
	gf.predicate,
	object_entity.canonical AS object_entity,
	object_entity.kind AS object_kind,
	gf.object_value,
	gf.valid_from,
	gf.valid_to,
	gf.updated_at,
	COALESCE(
		jsonb_agg(
			jsonb_build_object(
				'note_id', evidence.note_id,
				'status', note.status,
				'updated_at', note.updated_at
			)
			ORDER BY evidence.created_at ASC, evidence.note_id ASC
		) FILTER (WHERE evidence.note_id IS NOT NULL),
		'[]'::jsonb
	) AS evidence_notes
FROM graph_facts gf
JOIN graph_entities subject ON subject.entity_id = gf.subject_entity_id
LEFT JOIN graph_entities object_entity ON object_entity.entity_id = gf.object_entity_id
LEFT JOIN graph_fact_evidence evidence ON evidence.fact_id = gf.fact_id
LEFT JOIN memory_notes note ON note.note_id = evidence.note_id
WHERE gf.tenant_id = $1
	AND gf.project_id = $2
	AND gf.fact_id = ANY($3::uuid[])
GROUP BY
	gf.fact_id,
	gf.agent_id,
	gf.scope,
	subject.canonical,
	subject.kind,
	gf.predicate,
	object_entity.canonical,
	object_entity.kind,
	gf.object_value,
	gf.valid_from,
	gf.valid_to,
	gf.updated_at
ORDER BY gf.updated_at ASC, gf.fact_id ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(fact_ids)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}

/// Fetches applied proposal sources by identifier for a knowledge page rebuild.
pub async fn fetch_knowledge_proposal_sources<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	proposal_ids: &[Uuid],
) -> Result<Vec<KnowledgeProposalSource>>
where
	E: PgExecutor<'e>,
{
	if proposal_ids.is_empty() {
		return Ok(Vec::new());
	}

	let rows = sqlx::query_as::<_, KnowledgeProposalSource>(
		"\
SELECT
	proposal_id,
	run_id,
	agent_id,
	proposal_kind,
	apply_intent,
	review_state,
	source_refs,
	source_snapshot,
	lineage,
	diff,
	confidence,
	COALESCE(unsupported_claim_flags, '[]'::jsonb) AS unsupported_claim_flags,
	COALESCE(contradiction_markers, '[]'::jsonb) AS contradiction_markers,
	COALESCE(staleness_markers, '[]'::jsonb) AS staleness_markers,
	COALESCE(target_ref, '{}'::jsonb) AS target_ref,
	COALESCE(proposed_payload, '{}'::jsonb) AS proposed_payload,
	updated_at
FROM consolidation_proposals
WHERE tenant_id = $1
	AND project_id = $2
	AND proposal_id = ANY($3::uuid[])
	AND review_state = 'applied'
ORDER BY updated_at ASC, proposal_id ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(proposal_ids)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}
