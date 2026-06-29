use time::OffsetDateTime;

use super::{
	ELF_ENTITY_MEMORY_VIEW_SCHEMA_V1,
	build::{build_core_block_items, build_note_items, sort_entity_memory_items, summarize_items},
	storage::{
		fetch_aliases, fetch_entity_core_block_rows, fetch_entity_note_rows, resolve_entity,
	},
	types::{EntityMemoryEntity, EntityMemoryViewRequest, EntityMemoryViewResponse},
	validation::validate_entity_memory_request,
};
use crate::{ElfService, Result, access, search};

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
