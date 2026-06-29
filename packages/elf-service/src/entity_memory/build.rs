use std::collections::HashSet;

use time::OffsetDateTime;

use super::{
	TOP_OF_MIND_IMPORTANCE_THRESHOLD,
	storage::{EntityCoreBlockRow, EntityNoteRow},
	types::{EntityMemoryItem, EntityMemoryRelation, EntityMemorySummary},
};
use crate::access;

pub(super) fn build_note_items(
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

pub(super) fn build_core_block_items(
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

pub(super) fn note_lifecycle(
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

pub(super) fn note_read_bucket(lifecycle: &str, importance: f32) -> String {
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

pub(super) fn core_block_mentions_entity(row: &EntityCoreBlockRow, surfaces: &[String]) -> bool {
	let haystack =
		format!("{} {} {} {}", row.key, row.title, row.content, row.source_ref).to_lowercase();

	surfaces
		.iter()
		.map(|surface| surface.trim().to_lowercase())
		.filter(|surface| !surface.is_empty())
		.any(|surface| haystack.contains(surface.as_str()))
}

pub(super) fn summarize_items(items: &[EntityMemoryItem]) -> EntityMemorySummary {
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

pub(super) fn sort_entity_memory_items(items: &mut [EntityMemoryItem]) {
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
