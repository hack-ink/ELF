use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::graph::RelationTemporalStatus;

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
pub(super) struct PreparedEntityMemoryRequest {
	pub(super) tenant_id: String,
	pub(super) project_id: String,
	pub(super) agent_id: String,
	pub(super) read_profile: String,
	pub(super) entity_id: Option<Uuid>,
	pub(super) entity_surface: Option<String>,
}
