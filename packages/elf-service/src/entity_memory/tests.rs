use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	EntityMemoryItem,
	entity_memory::{
		build::{self, core_blocks, lifecycle},
		storage::EntityCoreBlockRow,
	},
};

#[test]
fn entity_memory_note_lifecycle_classifies_current_stale_superseded_and_tombstoned() {
	let as_of = OffsetDateTime::from_unix_timestamp(100).expect("valid timestamp");
	let expired = OffsetDateTime::from_unix_timestamp(90).expect("valid timestamp");

	assert_eq!(lifecycle::note_lifecycle("active", None, as_of), "current");
	assert_eq!(lifecycle::note_lifecycle("active", Some(expired), as_of), "stale");
	assert_eq!(lifecycle::note_lifecycle("deprecated", None, as_of), "superseded");
	assert_eq!(lifecycle::note_lifecycle("deleted", None, as_of), "tombstoned");
}

#[test]
fn entity_memory_read_bucket_keeps_only_current_high_importance_top_of_mind() {
	assert_eq!(lifecycle::note_read_bucket("current", 0.8), "top_of_mind");
	assert_eq!(lifecycle::note_read_bucket("current", 0.79), "background");
	assert_eq!(lifecycle::note_read_bucket("stale", 0.99), "background");
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

	assert!(core_blocks::core_block_mentions_entity(
		&row,
		&["Alice".to_string(), "Alicia".to_string()]
	));
	assert!(!core_blocks::core_block_mentions_entity(&row, &["Bob".to_string()]));
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
	let summary = build::summarize_items(&items);

	assert_eq!(summary.current_count, 1);
	assert_eq!(summary.stale_count, 1);
	assert_eq!(summary.top_of_mind_count, 1);
	assert_eq!(summary.background_count, 1);
	assert_eq!(summary.core_block_count, 1);
	assert_eq!(summary.archival_note_count, 1);
}
