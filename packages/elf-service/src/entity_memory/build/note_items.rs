use std::collections::HashSet;

use time::OffsetDateTime;

use crate::{
	access,
	entity_memory::{
		build::{lifecycle, visibility},
		storage::EntityNoteRow,
		types::{EntityMemoryItem, EntityMemoryRelation},
	},
	graph,
};

pub(in crate::entity_memory) fn build_note_items(
	rows: Vec<EntityNoteRow>,
	requester_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<access::SharedSpaceGrantKey>,
	as_of: OffsetDateTime,
) -> Vec<EntityMemoryItem> {
	let mut items = Vec::new();

	for row in rows {
		if !visibility::row_read_allowed(
			row.agent_id.as_str(),
			row.scope.as_str(),
			requester_agent_id,
			allowed_scopes,
			shared_grants,
		) || !visibility::row_read_allowed(
			row.fact_agent_id.as_str(),
			row.fact_scope.as_str(),
			requester_agent_id,
			allowed_scopes,
			shared_grants,
		) {
			continue;
		}

		let lifecycle = lifecycle::note_lifecycle(row.status.as_str(), row.expires_at, as_of);
		let read_bucket = lifecycle::note_read_bucket(lifecycle.as_str(), row.importance);
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

fn relation_from_note_row(row: &EntityNoteRow, as_of: OffsetDateTime) -> EntityMemoryRelation {
	EntityMemoryRelation {
		fact_id: row.fact_id,
		predicate: row.predicate.clone(),
		scope: row.fact_scope.clone(),
		actor: row.fact_agent_id.clone(),
		valid_from: row.valid_from,
		valid_to: row.valid_to,
		temporal_status: graph::relation_temporal_status(row.valid_from, row.valid_to, as_of),
	}
}
