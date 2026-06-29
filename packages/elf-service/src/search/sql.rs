pub(super) const SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1: &str = "search_retrieval_trajectory/v1";
pub(super) const SEARCH_FILTER_IMPACT_SCHEMA_V1: &str = "search_filter_impact/v1";
pub(super) const RECENT_TRACES_SCHEMA_V1: &str = "elf.recent_traces/v1";
pub(super) const TRACE_BUNDLE_SCHEMA_V1: &str = "elf.trace_bundle/v1";
pub(super) const MAX_RECENT_TRACES_LIMIT: u32 = 200;
pub(super) const DEFAULT_RECENT_TRACES_LIMIT: u32 = 50;
pub(super) const DEFAULT_BOUNDED_STAGE_ITEMS_LIMIT: u32 = 64;
pub(super) const DEFAULT_FULL_STAGE_ITEMS_LIMIT: u32 = 256;
pub(super) const DEFAULT_BOUNDED_CANDIDATES_LIMIT: u32 = 0;
pub(super) const DEFAULT_FULL_CANDIDATES_LIMIT: u32 = 200;
pub(super) const MAX_TRACE_BUNDLE_ITEMS_LIMIT: u32 = 256;
pub(super) const MAX_TRACE_BUNDLE_CANDIDATES_LIMIT: u32 = 1_000;
pub(super) const RELATION_CONTEXT_SQL: &str = r#"
WITH selected_facts AS (
	SELECT DISTINCT ON (snc.selected_note_id, gf.fact_id)
		snc.selected_note_id,
		gf.fact_id,
		gf.scope,
		subject_entity.canonical AS subject_canonical,
		subject_entity.kind AS subject_kind,
		gf.predicate,
		gf.object_entity_id,
		object_entity.canonical AS object_canonical,
		object_entity.kind AS object_kind,
		gf.object_value,
		gf.valid_from,
		gf.valid_to,
		(gf.valid_from <= $4 AND (gf.valid_to IS NULL OR gf.valid_to > $4)) AS is_current
	FROM unnest($7::uuid[]) AS snc(selected_note_id)
	JOIN memory_notes selected_note
		ON selected_note.note_id = snc.selected_note_id
	JOIN graph_fact_evidence gfe
		ON gfe.note_id = snc.selected_note_id
	JOIN graph_facts gf
		ON gf.fact_id = gfe.fact_id
	JOIN graph_entities subject_entity
		ON subject_entity.entity_id = gf.subject_entity_id
		AND subject_entity.tenant_id = $1
		AND subject_entity.project_id = $2
	LEFT JOIN graph_entities object_entity
		ON object_entity.entity_id = gf.object_entity_id
		AND object_entity.tenant_id = $1
		AND object_entity.project_id = $2
	WHERE gf.tenant_id = $1
		AND gf.project_id = $2
		AND selected_note.tenant_id = $1
		AND selected_note.project_id = $2
		AND selected_note.status = 'active'
		AND (
			selected_note.expires_at IS NULL
			OR selected_note.expires_at > $4
		)
		AND (
			($5 AND selected_note.scope = 'agent_private' AND selected_note.agent_id = $3)
			OR (
				selected_note.scope = ANY($6::text[])
				AND (
					selected_note.agent_id = $3
					OR concat(selected_note.scope, ':', selected_note.agent_id) = ANY($10::text[])
				)
			)
		)
		AND (
			($5 AND gf.scope = 'agent_private' AND gf.agent_id = $3)
			OR (
				gf.scope = ANY($6::text[])
				AND (
					gf.agent_id = $3
					OR concat(gf.scope, ':', gf.agent_id) = ANY($10::text[])
				)
			)
		)
		AND gf.valid_from <= $4
	ORDER BY
		snc.selected_note_id,
		gf.fact_id,
		(gf.valid_from <= $4 AND (gf.valid_to IS NULL OR gf.valid_to > $4)) DESC,
		gf.valid_from DESC,
		gf.fact_id ASC
),
ranked_facts AS (
	SELECT
		selected_note_id,
		fact_id,
		scope,
		subject_canonical,
		subject_kind,
		predicate,
		object_entity_id,
		object_canonical,
		object_kind,
		object_value,
		valid_from,
		valid_to,
		is_current,
		ROW_NUMBER() OVER (
			PARTITION BY selected_note_id
			ORDER BY is_current DESC, valid_from DESC, fact_id ASC
		) AS fact_rank
	FROM selected_facts
),
bounded_facts AS (
	SELECT
		selected_note_id,
		fact_id,
		scope,
		subject_canonical,
		subject_kind,
		predicate,
		object_entity_id,
		object_canonical,
		object_kind,
		object_value,
		valid_from,
		valid_to,
		is_current,
		fact_rank
	FROM ranked_facts
	WHERE fact_rank <= $9
),
evidence_ranked AS (
	SELECT
		bf.selected_note_id,
		bf.fact_id,
		bf.scope,
		bf.subject_canonical,
		bf.subject_kind,
		bf.predicate,
		bf.object_entity_id,
		bf.object_canonical,
		bf.object_kind,
		bf.object_value,
		bf.valid_from,
		bf.valid_to,
		bf.is_current,
		bf.fact_rank,
		e.note_id AS evidence_note_id,
		e.created_at AS evidence_created_at,
		ROW_NUMBER() OVER (
			PARTITION BY bf.selected_note_id, bf.fact_id
			ORDER BY e.created_at ASC, e.note_id ASC
		) AS evidence_rank
	FROM bounded_facts bf
	JOIN graph_fact_evidence e
		ON e.fact_id = bf.fact_id
	JOIN memory_notes evidence_note
		ON evidence_note.note_id = e.note_id
		AND evidence_note.tenant_id = $1
		AND evidence_note.project_id = $2
		AND evidence_note.status = 'active'
		AND (
			evidence_note.expires_at IS NULL
			OR evidence_note.expires_at > $4
		)
		AND (
			($5 AND evidence_note.scope = 'agent_private' AND evidence_note.agent_id = $3)
			OR (
				evidence_note.scope = ANY($6::text[])
				AND (
					evidence_note.agent_id = $3
					OR concat(evidence_note.scope, ':', evidence_note.agent_id) = ANY($10::text[])
				)
			)
		)
),
fact_contexts AS (
	SELECT
		selected_note_id,
		fact_id,
		scope,
		subject_canonical,
		subject_kind,
		predicate,
		object_entity_id,
		object_canonical,
		object_kind,
		object_value,
		valid_from,
		valid_to,
		is_current,
		fact_rank,
		ARRAY_AGG(evidence_note_id ORDER BY evidence_created_at ASC, evidence_note_id ASC) AS evidence_note_ids
	FROM evidence_ranked
	WHERE evidence_rank <= $8
	GROUP BY
		selected_note_id,
		fact_id,
		scope,
		subject_canonical,
		subject_kind,
		predicate,
		object_entity_id,
		object_canonical,
		object_kind,
		object_value,
		valid_from,
		valid_to,
		is_current,
		fact_rank
)
SELECT
	selected_note_id AS note_id,
	fact_id,
	scope,
	subject_canonical,
	subject_kind,
	predicate,
	object_entity_id,
	object_canonical,
	object_kind,
	object_value,
	valid_from,
	valid_to,
	is_current,
	evidence_note_ids
FROM fact_contexts
ORDER BY note_id, fact_rank
"#;
