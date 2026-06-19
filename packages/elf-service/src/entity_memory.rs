//! Entity-scoped memory authority readback.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgConnection, PgExecutor};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	ElfService, Error, Result,
	access::{self, ORG_PROJECT_ID},
	graph::RelationTemporalStatus,
	search,
};
use elf_storage::{graph, models::GraphEntity};

/// Entity memory view response schema identifier.
pub const ELF_ENTITY_MEMORY_VIEW_SCHEMA_V1: &str = "elf.entity_memory_view/v1";

const TOP_OF_MIND_IMPORTANCE_THRESHOLD: f32 = 0.8;

/// Request payload for an entity-scoped memory view.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EntityMemoryViewRequest {
	/// Tenant to query within.
	pub tenant_id: String,
	/// Project to query within.
	pub project_id: String,
	/// Agent requesting the read.
	pub agent_id: String,
	/// Read profile that determines visible scopes.
	pub read_profile: String,
	/// Exact graph entity id to resolve.
	pub entity_id: Option<Uuid>,
	/// Canonical or alias surface to resolve when entity_id is omitted.
	pub entity_surface: Option<String>,
}

/// Response payload for an entity-scoped memory view.
#[derive(Clone, Debug, Serialize)]
pub struct EntityMemoryViewResponse {
	/// Response schema identifier.
	pub schema: String,
	/// Tenant used for the read.
	pub tenant_id: String,
	/// Project used for the read.
	pub project_id: String,
	/// Agent that requested the read.
	pub agent_id: String,
	/// Read profile used for access control.
	pub read_profile: String,
	#[serde(with = "crate::time_serde")]
	/// Timestamp used for lifecycle classification.
	pub as_of: OffsetDateTime,
	/// Resolved graph entity.
	pub entity: EntityMemoryEntity,
	/// Aggregate counters for the returned items.
	pub summary: EntityMemorySummary,
	/// Entity-relevant core blocks and archival notes.
	pub items: Vec<EntityMemoryItem>,
}

/// Resolved graph entity reference.
#[derive(Clone, Debug, Serialize)]
pub struct EntityMemoryEntity {
	/// Entity identifier.
	pub entity_id: Uuid,
	/// Canonical entity surface.
	pub canonical: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional entity kind.
	pub kind: Option<String>,
	/// Canonical plus alias surfaces used for matching core blocks.
	pub surfaces: Vec<String>,
}

/// Aggregate counters for an entity memory view.
#[derive(Clone, Debug, Default, Serialize)]
pub struct EntityMemorySummary {
	/// Number of current items.
	pub current_count: usize,
	/// Number of stale items.
	pub stale_count: usize,
	/// Number of superseded items.
	pub superseded_count: usize,
	/// Number of tombstoned items.
	pub tombstoned_count: usize,
	/// Number of top-of-mind items.
	pub top_of_mind_count: usize,
	/// Number of background items.
	pub background_count: usize,
	/// Number of core memory block items.
	pub core_block_count: usize,
	/// Number of graph evidence note items.
	pub archival_note_count: usize,
}

/// One item in an entity memory view.
#[derive(Clone, Debug, Serialize)]
pub struct EntityMemoryItem {
	/// Source family for the item.
	pub source: String,
	/// Lifecycle bucket.
	pub lifecycle: String,
	/// Read bucket used by agents to decide whether to treat this as always-loaded context.
	pub read_bucket: String,
	/// Scope key for access explanation.
	pub scope: String,
	/// Agent that owns the source record.
	pub agent_id: String,
	/// Note identifier for archival_note items.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub note_id: Option<Uuid>,
	/// Core block identifier for core_block items.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub block_id: Option<Uuid>,
	/// Active core block attachment identifier for core_block items.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub attachment_id: Option<Uuid>,
	/// Optional note type discriminator.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub note_type: Option<String>,
	/// Optional stable source key.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub key: Option<String>,
	/// Human-readable title for core blocks.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub title: Option<String>,
	/// Text payload.
	pub text: String,
	/// Importance score when available.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub importance: Option<f32>,
	/// Confidence score when available.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub confidence: Option<f32>,
	/// Structured source/provenance metadata.
	pub source_ref: Value,
	#[serde(with = "crate::time_serde")]
	/// Last source update timestamp.
	pub updated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	/// Optional expiry timestamp for archival notes.
	pub expires_at: Option<OffsetDateTime>,
	/// Relations that connect this item to the entity.
	pub relations: Vec<EntityMemoryRelation>,
}

/// Graph relation that made an item relevant to the entity.
#[derive(Clone, Debug, Serialize)]
pub struct EntityMemoryRelation {
	/// Graph fact identifier.
	pub fact_id: Uuid,
	/// Predicate surface recorded on the fact.
	pub predicate: String,
	/// Scope of the graph fact.
	pub scope: String,
	/// Agent that emitted the graph fact.
	pub actor: String,
	#[serde(with = "crate::time_serde")]
	/// Start of fact validity window.
	pub valid_from: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	/// End of fact validity window, when superseded.
	pub valid_to: Option<OffsetDateTime>,
	/// Temporal state for the fact relative to the view timestamp.
	pub temporal_status: RelationTemporalStatus,
}

#[derive(Debug)]
struct PreparedEntityMemoryRequest {
	tenant_id: String,
	project_id: String,
	agent_id: String,
	read_profile: String,
	entity_id: Option<Uuid>,
	entity_surface: Option<String>,
}

#[derive(Clone, Debug, FromRow)]
struct EntityAliasRow {
	alias: String,
}

#[derive(Clone, Debug, FromRow)]
struct EntityNoteRow {
	note_id: Uuid,
	agent_id: String,
	scope: String,
	r#type: String,
	key: Option<String>,
	text: String,
	importance: f32,
	confidence: f32,
	status: String,
	updated_at: OffsetDateTime,
	expires_at: Option<OffsetDateTime>,
	source_ref: Value,
	fact_id: Uuid,
	fact_scope: String,
	fact_agent_id: String,
	predicate: String,
	valid_from: OffsetDateTime,
	valid_to: Option<OffsetDateTime>,
}

#[derive(Clone, Debug, FromRow)]
struct EntityCoreBlockRow {
	attachment_id: Uuid,
	block_id: Uuid,
	agent_id: String,
	scope: String,
	key: String,
	title: String,
	content: String,
	source_ref: Value,
	updated_at: OffsetDateTime,
}

impl ElfService {
	/// Returns an entity-scoped view across attached core blocks and graph-linked notes.
	pub async fn entity_memory_view(
		&self,
		req: EntityMemoryViewRequest,
	) -> Result<EntityMemoryViewResponse> {
		let prepared = validate_entity_memory_request(req)?;
		let allowed_scopes =
			search::resolve_read_profile_scopes(&self.cfg, prepared.read_profile.as_str())?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let as_of = OffsetDateTime::now_utc();
		let mut conn = self.db.pool.acquire().await?;
		let entity = resolve_entity(&mut conn, &prepared).await?;
		let aliases = fetch_aliases(conn.as_mut(), entity.entity_id).await?;
		let mut surfaces = vec![entity.canonical.clone()];

		for alias in aliases {
			if !surfaces.iter().any(|surface| surface.eq_ignore_ascii_case(&alias)) {
				surfaces.push(alias);
			}
		}

		let shared_grants = access::load_shared_read_grants_with_org_shared(
			conn.as_mut(),
			prepared.tenant_id.as_str(),
			prepared.project_id.as_str(),
			prepared.agent_id.as_str(),
			org_shared_allowed,
		)
		.await?;
		let note_rows = fetch_entity_note_rows(
			conn.as_mut(),
			prepared.tenant_id.as_str(),
			prepared.project_id.as_str(),
			entity.entity_id,
			&allowed_scopes,
		)
		.await?;
		let block_rows = fetch_entity_core_block_rows(
			conn.as_mut(),
			prepared.tenant_id.as_str(),
			prepared.project_id.as_str(),
			prepared.agent_id.as_str(),
			prepared.read_profile.as_str(),
		)
		.await?;
		let mut items = build_note_items(
			note_rows,
			prepared.agent_id.as_str(),
			&allowed_scopes,
			&shared_grants,
			as_of,
		);

		items.extend(build_core_block_items(
			block_rows,
			prepared.agent_id.as_str(),
			&allowed_scopes,
			&shared_grants,
			&surfaces,
		));

		sort_entity_memory_items(&mut items);

		let summary = summarize_items(&items);

		Ok(EntityMemoryViewResponse {
			schema: ELF_ENTITY_MEMORY_VIEW_SCHEMA_V1.to_string(),
			tenant_id: prepared.tenant_id,
			project_id: prepared.project_id,
			agent_id: prepared.agent_id,
			read_profile: prepared.read_profile,
			as_of,
			entity: EntityMemoryEntity {
				entity_id: entity.entity_id,
				canonical: entity.canonical,
				kind: entity.kind,
				surfaces,
			},
			summary,
			items,
		})
	}
}

fn validate_entity_memory_request(
	req: EntityMemoryViewRequest,
) -> Result<PreparedEntityMemoryRequest> {
	let tenant_id = normalize_required(req.tenant_id.as_str(), "tenant_id")?;
	let project_id = normalize_required(req.project_id.as_str(), "project_id")?;
	let agent_id = normalize_required(req.agent_id.as_str(), "agent_id")?;
	let read_profile = normalize_required(req.read_profile.as_str(), "read_profile")?;
	let entity_surface = req
		.entity_surface
		.as_deref()
		.map(|surface| normalize_required(surface, "entity_surface"))
		.transpose()?;

	if req.entity_id.is_some() == entity_surface.is_some() {
		return Err(Error::InvalidRequest {
			message: "Exactly one of entity_id or entity_surface is required.".to_string(),
		});
	}

	Ok(PreparedEntityMemoryRequest {
		tenant_id,
		project_id,
		agent_id,
		read_profile,
		entity_id: req.entity_id,
		entity_surface,
	})
}

fn build_note_items(
	rows: Vec<EntityNoteRow>,
	requester_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<access::SharedSpaceGrantKey>,
	as_of: OffsetDateTime,
) -> Vec<EntityMemoryItem> {
	let mut items = Vec::new();

	for row in rows {
		if !row_read_allowed(
			row.agent_id.as_str(),
			row.scope.as_str(),
			requester_agent_id,
			allowed_scopes,
			shared_grants,
		) || !row_read_allowed(
			row.fact_agent_id.as_str(),
			row.fact_scope.as_str(),
			requester_agent_id,
			allowed_scopes,
			shared_grants,
		) {
			continue;
		}

		let lifecycle = note_lifecycle(row.status.as_str(), row.expires_at, as_of);
		let read_bucket = note_read_bucket(lifecycle.as_str(), row.importance);
		let relation = relation_from_note_row(&row, as_of);

		if let Some(item) = items.iter_mut().find(|item: &&mut EntityMemoryItem| {
			item.source == "archival_note" && item.note_id == Some(row.note_id)
		}) {
			item.relations.push(relation);

			continue;
		}

		items.push(EntityMemoryItem {
			source: "archival_note".to_string(),
			lifecycle,
			read_bucket,
			scope: row.scope,
			agent_id: row.agent_id,
			note_id: Some(row.note_id),
			block_id: None,
			attachment_id: None,
			note_type: Some(row.r#type),
			key: row.key,
			title: None,
			text: row.text,
			importance: Some(row.importance),
			confidence: Some(row.confidence),
			source_ref: row.source_ref,
			updated_at: row.updated_at,
			expires_at: row.expires_at,
			relations: vec![relation],
		});
	}

	items
}

fn build_core_block_items(
	rows: Vec<EntityCoreBlockRow>,
	requester_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<access::SharedSpaceGrantKey>,
	surfaces: &[String],
) -> Vec<EntityMemoryItem> {
	rows.into_iter()
		.filter(|row| {
			row_read_allowed(
				row.agent_id.as_str(),
				row.scope.as_str(),
				requester_agent_id,
				allowed_scopes,
				shared_grants,
			) && core_block_mentions_entity(row, surfaces)
		})
		.map(|row| EntityMemoryItem {
			source: "core_block".to_string(),
			lifecycle: "current".to_string(),
			read_bucket: "top_of_mind".to_string(),
			scope: row.scope,
			agent_id: row.agent_id,
			note_id: None,
			block_id: Some(row.block_id),
			attachment_id: Some(row.attachment_id),
			note_type: None,
			key: Some(row.key),
			title: Some(row.title),
			text: row.content,
			importance: None,
			confidence: None,
			source_ref: row.source_ref,
			updated_at: row.updated_at,
			expires_at: None,
			relations: Vec::new(),
		})
		.collect()
}

fn row_read_allowed(
	owner_agent_id: &str,
	scope: &str,
	requester_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<access::SharedSpaceGrantKey>,
) -> bool {
	if !allowed_scopes.iter().any(|allowed| allowed == scope) {
		return false;
	}
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

fn note_lifecycle(
	status: &str,
	expires_at: Option<OffsetDateTime>,
	as_of: OffsetDateTime,
) -> String {
	match status {
		"active" if expires_at.is_some_and(|expires_at| expires_at <= as_of) => "stale".to_string(),
		"active" => "current".to_string(),
		"deprecated" => "superseded".to_string(),
		"deleted" => "tombstoned".to_string(),
		other => other.to_string(),
	}
}

fn note_read_bucket(lifecycle: &str, importance: f32) -> String {
	if lifecycle == "current" && importance >= TOP_OF_MIND_IMPORTANCE_THRESHOLD {
		"top_of_mind".to_string()
	} else {
		"background".to_string()
	}
}

fn relation_from_note_row(row: &EntityNoteRow, as_of: OffsetDateTime) -> EntityMemoryRelation {
	EntityMemoryRelation {
		fact_id: row.fact_id,
		predicate: row.predicate.clone(),
		scope: row.fact_scope.clone(),
		actor: row.fact_agent_id.clone(),
		valid_from: row.valid_from,
		valid_to: row.valid_to,
		temporal_status: crate::graph::relation_temporal_status(
			row.valid_from,
			row.valid_to,
			as_of,
		),
	}
}

fn core_block_mentions_entity(row: &EntityCoreBlockRow, surfaces: &[String]) -> bool {
	let haystack =
		format!("{} {} {} {}", row.key, row.title, row.content, row.source_ref).to_lowercase();

	surfaces
		.iter()
		.map(|surface| surface.trim().to_lowercase())
		.filter(|surface| !surface.is_empty())
		.any(|surface| haystack.contains(surface.as_str()))
}

fn summarize_items(items: &[EntityMemoryItem]) -> EntityMemorySummary {
	let mut summary = EntityMemorySummary::default();

	for item in items {
		match item.lifecycle.as_str() {
			"current" => summary.current_count += 1,
			"stale" => summary.stale_count += 1,
			"superseded" => summary.superseded_count += 1,
			"tombstoned" => summary.tombstoned_count += 1,
			_ => {},
		}
		match item.read_bucket.as_str() {
			"top_of_mind" => summary.top_of_mind_count += 1,
			"background" => summary.background_count += 1,
			_ => {},
		}
		match item.source.as_str() {
			"core_block" => summary.core_block_count += 1,
			"archival_note" => summary.archival_note_count += 1,
			_ => {},
		}
	}

	summary
}

fn sort_entity_memory_items(items: &mut [EntityMemoryItem]) {
	items.sort_by(|left, right| {
		read_bucket_rank(right.read_bucket.as_str())
			.cmp(&read_bucket_rank(left.read_bucket.as_str()))
			.then_with(|| {
				lifecycle_rank(right.lifecycle.as_str())
					.cmp(&lifecycle_rank(left.lifecycle.as_str()))
			})
			.then_with(|| right.updated_at.cmp(&left.updated_at))
			.then_with(|| left.source.cmp(&right.source))
	});
}

fn read_bucket_rank(bucket: &str) -> u8 {
	match bucket {
		"top_of_mind" => 1,
		_ => 0,
	}
}

fn lifecycle_rank(lifecycle: &str) -> u8 {
	match lifecycle {
		"current" => 3,
		"stale" => 2,
		"superseded" => 1,
		_ => 0,
	}
}

fn normalize_required(raw: &str, field: &str) -> Result<String> {
	let trimmed = raw.trim();

	if trimmed.is_empty() {
		return Err(Error::InvalidRequest { message: format!("{field} is required.") });
	}

	Ok(trimmed.to_string())
}

async fn resolve_entity(
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

async fn fetch_aliases<'e, E>(executor: E, entity_id: Uuid) -> Result<Vec<String>>
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

async fn fetch_entity_note_rows<'e, E>(
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

async fn fetch_entity_core_block_rows<'e, E>(
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

#[cfg(test)]
mod tests {
	use serde_json;
	use time::OffsetDateTime;
	use uuid::Uuid;

	use crate::{
		EntityMemoryItem,
		entity_memory::{self, EntityCoreBlockRow},
	};

	#[test]
	fn entity_memory_note_lifecycle_classifies_current_stale_superseded_and_tombstoned() {
		let as_of = OffsetDateTime::from_unix_timestamp(100).expect("valid timestamp");
		let expired = OffsetDateTime::from_unix_timestamp(90).expect("valid timestamp");

		assert_eq!(entity_memory::note_lifecycle("active", None, as_of), "current");
		assert_eq!(entity_memory::note_lifecycle("active", Some(expired), as_of), "stale");
		assert_eq!(entity_memory::note_lifecycle("deprecated", None, as_of), "superseded");
		assert_eq!(entity_memory::note_lifecycle("deleted", None, as_of), "tombstoned");
	}

	#[test]
	fn entity_memory_read_bucket_keeps_only_current_high_importance_top_of_mind() {
		assert_eq!(entity_memory::note_read_bucket("current", 0.8), "top_of_mind");
		assert_eq!(entity_memory::note_read_bucket("current", 0.79), "background");
		assert_eq!(entity_memory::note_read_bucket("stale", 0.99), "background");
	}

	#[test]
	fn entity_memory_core_block_mentions_canonical_or_alias_surface() {
		let row = EntityCoreBlockRow {
			attachment_id: Uuid::from_u128(1),
			block_id: Uuid::from_u128(2),
			agent_id: "agent".to_string(),
			scope: "agent_private".to_string(),
			key: "preferences".to_string(),
			title: "Profile".to_string(),
			content: "Alicia prefers precise architecture notes.".to_string(),
			source_ref: serde_json::json!({ "source": "core" }),
			updated_at: OffsetDateTime::from_unix_timestamp(100).expect("valid timestamp"),
		};

		assert!(entity_memory::core_block_mentions_entity(
			&row,
			&["Alice".to_string(), "Alicia".to_string()]
		));
		assert!(!entity_memory::core_block_mentions_entity(&row, &["Bob".to_string()]));
	}

	#[test]
	fn entity_memory_summary_counts_lifecycle_and_read_buckets() {
		let now = OffsetDateTime::from_unix_timestamp(100).expect("valid timestamp");
		let items = vec![
			EntityMemoryItem {
				source: "core_block".to_string(),
				lifecycle: "current".to_string(),
				read_bucket: "top_of_mind".to_string(),
				scope: "agent_private".to_string(),
				agent_id: "agent".to_string(),
				note_id: None,
				block_id: Some(Uuid::from_u128(1)),
				attachment_id: Some(Uuid::from_u128(2)),
				note_type: None,
				key: Some("profile".to_string()),
				title: Some("Profile".to_string()),
				text: "Alice prefers concise updates.".to_string(),
				importance: None,
				confidence: None,
				source_ref: serde_json::json!({}),
				updated_at: now,
				expires_at: None,
				relations: Vec::new(),
			},
			EntityMemoryItem {
				source: "archival_note".to_string(),
				lifecycle: "stale".to_string(),
				read_bucket: "background".to_string(),
				scope: "project_shared".to_string(),
				agent_id: "agent".to_string(),
				note_id: Some(Uuid::from_u128(3)),
				block_id: None,
				attachment_id: None,
				note_type: Some("preference".to_string()),
				key: None,
				title: None,
				text: "Alice once preferred verbose updates.".to_string(),
				importance: Some(0.7),
				confidence: Some(0.9),
				source_ref: serde_json::json!({}),
				updated_at: now,
				expires_at: Some(now),
				relations: Vec::new(),
			},
		];
		let summary = entity_memory::summarize_items(&items);

		assert_eq!(summary.current_count, 1);
		assert_eq!(summary.stale_count, 1);
		assert_eq!(summary.top_of_mind_count, 1);
		assert_eq!(summary.background_count, 1);
		assert_eq!(summary.core_block_count, 1);
		assert_eq!(summary.archival_note_count, 1);
	}
}
