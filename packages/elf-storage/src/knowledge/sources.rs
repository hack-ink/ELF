use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{
	Result,
	knowledge::types::{
		KnowledgeDocChunkSource, KnowledgeDocSource, KnowledgeEventSource, KnowledgeNoteSource,
		KnowledgeProposalSource, KnowledgeRelationSource, KnowledgeRelationSourcesFetch,
	},
};

/// Fetches note sources by identifier for a knowledge page rebuild.
pub async fn fetch_knowledge_note_sources<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	agent_id: Option<&str>,
	allowed_scopes: &[String],
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
	AND ($3::text IS NULL OR scope <> 'agent_private' OR agent_id = $3)
	AND scope = ANY($4::text[])
	AND note_id = ANY($5::uuid[])
	AND status = 'active'
	AND (expires_at IS NULL OR expires_at > now())
ORDER BY updated_at ASC, note_id ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(agent_id)
	.bind(allowed_scopes)
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
	agent_id: Option<&str>,
	allowed_scopes: &[String],
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
	memory_ingest_decisions.decision_id,
	memory_ingest_decisions.agent_id,
	memory_ingest_decisions.scope,
	memory_ingest_decisions.pipeline,
	memory_ingest_decisions.note_type,
	memory_ingest_decisions.note_key,
	memory_ingest_decisions.note_id,
	memory_ingest_decisions.policy_decision,
	memory_ingest_decisions.note_op,
	memory_ingest_decisions.reason_code,
	memory_ingest_decisions.details,
	memory_ingest_decisions.ts
FROM memory_ingest_decisions
JOIN memory_notes note ON note.note_id = memory_ingest_decisions.note_id
WHERE memory_ingest_decisions.tenant_id = $1
	AND memory_ingest_decisions.project_id = $2
	AND ($3::text IS NULL OR memory_ingest_decisions.scope <> 'agent_private' OR memory_ingest_decisions.agent_id = $3)
	AND memory_ingest_decisions.scope = ANY($4::text[])
	AND memory_ingest_decisions.decision_id = ANY($5::uuid[])
	AND memory_ingest_decisions.pipeline = 'add_event'
	AND memory_ingest_decisions.policy_decision IN ('remember', 'update')
	AND note.tenant_id = memory_ingest_decisions.tenant_id
	AND note.project_id = memory_ingest_decisions.project_id
	AND note.status = 'active'
	AND (note.expires_at IS NULL OR note.expires_at > now())
	AND ($3::text IS NULL OR note.scope <> 'agent_private' OR note.agent_id = $3)
	AND note.scope = ANY($4::text[])
ORDER BY memory_ingest_decisions.ts ASC, memory_ingest_decisions.decision_id ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(agent_id)
	.bind(allowed_scopes)
	.bind(decision_ids)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}

/// Fetches relation sources by graph fact identifier for a knowledge page rebuild.
pub async fn fetch_knowledge_relation_sources<'e, E>(
	executor: E,
	params: KnowledgeRelationSourcesFetch<'_>,
) -> Result<Vec<KnowledgeRelationSource>>
where
	E: PgExecutor<'e>,
{
	if params.fact_ids.is_empty() {
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
		) FILTER (
			WHERE evidence.note_id IS NOT NULL
				AND note.tenant_id = gf.tenant_id
				AND note.project_id = gf.project_id
				AND note.status = 'active'
				AND (note.expires_at IS NULL OR note.expires_at > now())
				AND note.scope = ANY($4::text[])
				AND (
					$3::text IS NULL
					OR ($6 AND note.scope = 'agent_private' AND note.agent_id = $3)
					OR (
						note.scope <> 'agent_private'
						AND (
							note.agent_id = $3
							OR concat(note.scope, ':', note.agent_id) = ANY($5::text[])
						)
					)
				)
		),
		'[]'::jsonb
	) AS evidence_notes
FROM graph_facts gf
JOIN graph_entities subject ON subject.entity_id = gf.subject_entity_id
LEFT JOIN graph_entities object_entity ON object_entity.entity_id = gf.object_entity_id
LEFT JOIN graph_fact_evidence evidence ON evidence.fact_id = gf.fact_id
LEFT JOIN memory_notes note ON note.note_id = evidence.note_id
WHERE gf.tenant_id = $1
	AND gf.project_id = $2
	AND gf.scope = ANY($4::text[])
	AND (
		$3::text IS NULL
		OR ($6 AND gf.scope = 'agent_private' AND gf.agent_id = $3)
		OR (
			gf.scope <> 'agent_private'
			AND (
				gf.agent_id = $3
				OR concat(gf.scope, ':', gf.agent_id) = ANY($5::text[])
			)
		)
	)
	AND gf.fact_id = ANY($7::uuid[])
	AND EXISTS (
		SELECT 1
		FROM graph_fact_evidence readable_evidence
		JOIN memory_notes readable_note
			ON readable_note.note_id = readable_evidence.note_id
		WHERE readable_evidence.fact_id = gf.fact_id
			AND readable_note.tenant_id = gf.tenant_id
			AND readable_note.project_id = gf.project_id
			AND readable_note.status = 'active'
			AND (readable_note.expires_at IS NULL OR readable_note.expires_at > now())
			AND readable_note.scope = ANY($4::text[])
			AND (
				$3::text IS NULL
				OR ($6 AND readable_note.scope = 'agent_private' AND readable_note.agent_id = $3)
				OR (
					readable_note.scope <> 'agent_private'
					AND (
						readable_note.agent_id = $3
						OR concat(readable_note.scope, ':', readable_note.agent_id) = ANY($5::text[])
					)
				)
			)
	)
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
	.bind(params.tenant_id)
	.bind(params.project_id)
	.bind(params.agent_id)
	.bind(params.allowed_scopes)
	.bind(params.shared_scope_keys)
	.bind(params.private_allowed)
	.bind(params.fact_ids)
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

/// Fetches active Source Library documents by identifier for a knowledge page rebuild.
pub async fn fetch_knowledge_doc_sources<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	agent_id: Option<&str>,
	allowed_scopes: &[String],
	doc_ids: &[Uuid],
) -> Result<Vec<KnowledgeDocSource>>
where
	E: PgExecutor<'e>,
{
	if doc_ids.is_empty() {
		return Ok(Vec::new());
	}

	let rows = sqlx::query_as::<_, KnowledgeDocSource>(
		"\
SELECT
	doc_id,
	agent_id,
	scope,
	doc_type,
	status,
	title,
	COALESCE(source_ref, '{}'::jsonb) AS source_ref,
	content,
	content_bytes,
	content_hash,
	created_at,
	updated_at
FROM doc_documents
WHERE tenant_id = $1
	AND project_id = $2
	AND ($3::text IS NULL OR scope <> 'agent_private' OR agent_id = $3)
	AND scope = ANY($4::text[])
	AND doc_id = ANY($5::uuid[])
	AND status = 'active'
ORDER BY updated_at ASC, doc_id ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(agent_id)
	.bind(allowed_scopes)
	.bind(doc_ids)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}

/// Fetches active Source Library document chunks by identifier for a knowledge page rebuild.
pub async fn fetch_knowledge_doc_chunk_sources<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	agent_id: Option<&str>,
	allowed_scopes: &[String],
	chunk_ids: &[Uuid],
) -> Result<Vec<KnowledgeDocChunkSource>>
where
	E: PgExecutor<'e>,
{
	if chunk_ids.is_empty() {
		return Ok(Vec::new());
	}

	let rows = sqlx::query_as::<_, KnowledgeDocChunkSource>(
		"\
SELECT
	c.chunk_id,
	c.doc_id,
	d.agent_id,
	d.scope,
	d.doc_type,
	d.status,
	d.title,
	COALESCE(d.source_ref, '{}'::jsonb) AS source_ref,
	d.content_hash AS doc_content_hash,
	d.updated_at AS doc_updated_at,
	c.chunk_index,
	c.start_offset,
	c.end_offset,
	c.chunk_text,
	c.chunk_hash,
	c.created_at AS chunk_created_at
FROM doc_chunks c
JOIN doc_documents d ON d.doc_id = c.doc_id
WHERE d.tenant_id = $1
	AND d.project_id = $2
	AND ($3::text IS NULL OR d.scope <> 'agent_private' OR d.agent_id = $3)
	AND d.scope = ANY($4::text[])
	AND c.chunk_id = ANY($5::uuid[])
	AND d.status = 'active'
ORDER BY d.updated_at ASC, c.chunk_index ASC, c.chunk_id ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(agent_id)
	.bind(allowed_scopes)
	.bind(chunk_ids)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}
