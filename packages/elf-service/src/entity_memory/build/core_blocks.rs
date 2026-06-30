use std::collections::HashSet;

use crate::{
	access,
	entity_memory::{build::visibility, storage::EntityCoreBlockRow, types::EntityMemoryItem},
};

pub(in crate::entity_memory) fn build_core_block_items(
	rows: Vec<EntityCoreBlockRow>,
	requester_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<access::SharedSpaceGrantKey>,
	surfaces: &[String],
) -> Vec<EntityMemoryItem> {
	rows.into_iter()
		.filter(|row| {
			visibility::row_read_allowed(
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

pub(in crate::entity_memory) fn core_block_mentions_entity(
	row: &EntityCoreBlockRow,
	surfaces: &[String],
) -> bool {
	let haystack =
		format!("{} {} {} {}", row.key, row.title, row.content, row.source_ref).to_lowercase();

	surfaces
		.iter()
		.map(|surface| surface.trim().to_lowercase())
		.filter(|surface| !surface.is_empty())
		.any(|surface| haystack.contains(surface.as_str()))
}
