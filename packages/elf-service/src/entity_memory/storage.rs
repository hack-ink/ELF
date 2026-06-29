use serde_json::Value;
use sqlx::{FromRow, PgConnection, PgExecutor};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	Error, Result, access::ORG_PROJECT_ID, entity_memory::types::PreparedEntityMemoryRequest,
};
use elf_storage::{graph, models::GraphEntity};

#[derive(Clone, Debug, FromRow)]
pub(super) struct EntityNoteRow {
	pub(super) note_id: Uuid,
	pub(super) agent_id: String,
	pub(super) scope: String,
	pub(super) r#type: String,
	pub(super) key: Option<String>,
	pub(super) text: String,
	pub(super) importance: f32,
	pub(super) confidence: f32,
	pub(super) status: String,
	pub(super) updated_at: OffsetDateTime,
	pub(super) expires_at: Option<OffsetDateTime>,
	pub(super) source_ref: Value,
	pub(super) fact_id: Uuid,
	pub(super) fact_scope: String,
	pub(super) fact_agent_id: String,
	pub(super) predicate: String,
	pub(super) valid_from: OffsetDateTime,
	pub(super) valid_to: Option<OffsetDateTime>,
}

#[derive(Clone, Debug, FromRow)]
pub(super) struct EntityCoreBlockRow {
	pub(super) attachment_id: Uuid,
	pub(super) block_id: Uuid,
	pub(super) agent_id: String,
	pub(super) scope: String,
	pub(super) key: String,
	pub(super) title: String,
	pub(super) content: String,
	pub(super) source_ref: Value,
	pub(super) updated_at: OffsetDateTime,
}

#[derive(Clone, Debug, FromRow)]
struct EntityAliasRow {
	alias: String,
}

pub(super) async fn resolve_entity(
	conn: &mut PgConnection,
	req: &PreparedEntityMemoryRequest,
) -> Result<GraphEntity> {
	if let Some(entity_id) = req.entity_id {
		return sqlx::query_as::<_, GraphEntity>(
			"\
SELECT
	entity_id,
	tenant_id,
	project_id,
	canonical,
	canonical_norm,
	kind,
	created_at,
	updated_at
FROM graph_entities
WHERE tenant_id = $1
	AND project_id = $2
	AND entity_id = $3",
		)
		.bind(req.tenant_id.as_str())
		.bind(req.project_id.as_str())
		.bind(entity_id)
		.fetch_optional(conn)
		.await?
		.ok_or_else(|| Error::NotFound {
			message: format!("graph entity not found: {entity_id}"),
		});
	}

	let surface = req.entity_surface.as_deref().expect("surface was validated");

	graph::resolve_entity_by_surface(conn, req.tenant_id.as_str(), req.project_id.as_str(), surface)
		.await
		.map_err(|err| Error::Storage { message: err.to_string() })?
		.ok_or_else(|| Error::NotFound {
			message: format!("graph entity not found for surface={surface}"),
		})
}

pub(super) async fn fetch_aliases<'e, E>(executor: E, entity_id: Uuid) -> Result<Vec<String>>
where
	E: PgExecutor<'e>,
{
	let rows = sqlx::query_as::<_, EntityAliasRow>(
		"\
SELECT alias
FROM graph_entity_aliases
WHERE entity_id = $1
ORDER BY alias ASC",
	)
	.bind(entity_id)
	.fetch_all(executor)
	.await?;

	Ok(rows.into_iter().map(|row| row.alias).collect())
}

pub(super) async fn fetch_entity_note_rows<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	entity_id: Uuid,
	allowed_scopes: &[String],
) -> Result<Vec<EntityNoteRow>>
where
	E: PgExecutor<'e>,
{
	sqlx::query_as::<_, EntityNoteRow>(
		"\
SELECT
	n.note_id,
	n.agent_id,
	n.scope,
	n.type,
	n.key,
	n.text,
	n.importance,
	n.confidence,
	n.status,
	n.updated_at,
	n.expires_at,
	n.source_ref,
	gf.fact_id,
	gf.scope AS fact_scope,
	gf.agent_id AS fact_agent_id,
	gf.predicate,
	gf.valid_from,
	gf.valid_to
FROM graph_facts gf
JOIN graph_fact_evidence gfe ON gfe.fact_id = gf.fact_id
JOIN memory_notes n ON n.note_id = gfe.note_id
WHERE gf.tenant_id = $1
	AND (gf.project_id = $2 OR (gf.project_id = $5 AND gf.scope = 'org_shared'))
	AND (gf.subject_entity_id = $3 OR gf.object_entity_id = $3)
	AND gf.scope = ANY($4::text[])
	AND n.tenant_id = $1
	AND (n.project_id = $2 OR (n.project_id = $5 AND n.scope = 'org_shared'))
	AND n.scope = ANY($4::text[])
ORDER BY n.updated_at DESC, n.note_id ASC, gf.valid_from DESC, gf.fact_id ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(entity_id)
	.bind(allowed_scopes)
	.bind(ORG_PROJECT_ID)
	.fetch_all(executor)
	.await
	.map_err(Into::into)
}

pub(super) async fn fetch_entity_core_block_rows<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	read_profile: &str,
) -> Result<Vec<EntityCoreBlockRow>>
where
	E: PgExecutor<'e>,
{
	sqlx::query_as::<_, EntityCoreBlockRow>(
		"\
SELECT
	a.attachment_id,
	b.block_id,
	b.agent_id,
	b.scope,
	b.key,
	b.title,
	b.content,
	b.source_ref,
	b.updated_at
FROM core_memory_block_attachments a
JOIN core_memory_blocks b ON b.block_id = a.block_id
WHERE a.tenant_id = $1
	AND a.project_id = $2
	AND a.agent_id = $3
	AND a.read_profile = $4
	AND a.detached_at IS NULL
	AND b.status = 'active'
ORDER BY a.attached_at ASC, b.key ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(agent_id)
	.bind(read_profile)
	.fetch_all(executor)
	.await
	.map_err(Into::into)
}
