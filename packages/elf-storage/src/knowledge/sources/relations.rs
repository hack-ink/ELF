use sqlx::PgExecutor;

use crate::{
	Result,
	knowledge::types::{KnowledgeRelationSource, KnowledgeRelationSourcesFetch},
};

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
