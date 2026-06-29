use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

/// Persisted graph entity row.
#[derive(Debug, FromRow)]
pub struct GraphEntity {
	/// Entity identifier.
	pub entity_id: Uuid,
	/// Tenant that owns the entity.
	pub tenant_id: String,
	/// Project that owns the entity.
	pub project_id: String,
	/// Canonical entity surface.
	pub canonical: String,
	/// Normalized canonical entity surface.
	pub canonical_norm: String,
	/// Optional entity kind.
	pub kind: Option<String>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Persisted alias row for a graph entity.
#[derive(Debug, FromRow)]
pub struct GraphEntityAlias {
	/// Alias identifier.
	pub alias_id: Uuid,
	/// Entity identifier that owns the alias.
	pub entity_id: Uuid,
	/// Alias surface.
	pub alias: String,
	/// Normalized alias surface.
	pub alias_norm: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted graph fact row.
#[derive(Debug, FromRow)]
pub struct GraphFact {
	/// Fact identifier.
	pub fact_id: Uuid,
	/// Tenant that owns the fact.
	pub tenant_id: String,
	/// Project that owns the fact.
	pub project_id: String,
	/// Agent that emitted the fact.
	pub agent_id: String,
	/// Scope key for the fact.
	pub scope: String,
	/// Subject entity identifier.
	pub subject_entity_id: Uuid,
	/// Predicate surface captured with the fact.
	pub predicate: String,
	/// Resolved predicate identifier, when available.
	pub predicate_id: Option<Uuid>,
	/// Object entity identifier for entity-to-entity facts.
	pub object_entity_id: Option<Uuid>,
	/// Scalar object value for entity-to-value facts.
	pub object_value: Option<String>,
	/// Start of the fact validity window.
	pub valid_from: OffsetDateTime,
	/// End of the fact validity window, if superseded.
	pub valid_to: Option<OffsetDateTime>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Evidence link between one graph fact and one memory note.
#[derive(Debug, FromRow)]
pub struct GraphFactEvidence {
	/// Evidence row identifier.
	pub evidence_id: Uuid,
	/// Fact identifier.
	pub fact_id: Uuid,
	/// Note identifier that supports the fact.
	pub note_id: Uuid,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted graph predicate row.
#[derive(Debug, FromRow)]
pub struct GraphPredicate {
	/// Predicate identifier.
	pub predicate_id: Uuid,
	/// Scope key where the predicate is visible.
	pub scope_key: String,
	/// Tenant scope, when tenant-specific.
	pub tenant_id: Option<String>,
	/// Project scope, when project-specific.
	pub project_id: Option<String>,
	/// Canonical predicate surface.
	pub canonical: String,
	/// Normalized canonical predicate surface.
	pub canonical_norm: String,
	/// Cardinality policy for the predicate.
	pub cardinality: String,
	/// Lifecycle status for the predicate.
	pub status: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Persisted alias row for a graph predicate.
#[derive(Debug, FromRow)]
pub struct GraphPredicateAlias {
	/// Alias identifier.
	pub alias_id: Uuid,
	/// Predicate identifier that owns the alias.
	pub predicate_id: Uuid,
	/// Scope key where the alias resolves.
	pub scope_key: String,
	/// Alias surface.
	pub alias: String,
	/// Normalized alias surface.
	pub alias_norm: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted supersession row linking two facts.
#[derive(Debug, FromRow)]
pub struct GraphFactSupersession {
	/// Supersession identifier.
	pub supersession_id: Uuid,
	/// Tenant that owns the supersession record.
	pub tenant_id: String,
	/// Project that owns the supersession record.
	pub project_id: String,
	/// Fact identifier that was superseded.
	pub from_fact_id: Uuid,
	/// Fact identifier that replaced the prior fact.
	pub to_fact_id: Uuid,
	/// Note identifier that justified the supersession.
	pub note_id: Uuid,
	/// Time the supersession took effect.
	pub effective_at: OffsetDateTime,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}
