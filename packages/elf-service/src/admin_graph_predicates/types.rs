use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

/// Request payload for listing graph predicates visible in admin scope.
#[derive(Clone, Debug)]
pub struct AdminGraphPredicatesListRequest {
	/// Tenant to query within.
	pub tenant_id: String,
	/// Project to query within.
	pub project_id: String,
	/// Agent requesting the list.
	pub agent_id: String,
	/// Optional admin scope filter.
	pub scope: Option<String>,
}

/// Request payload for patching a graph predicate.
#[derive(Clone, Debug)]
pub struct AdminGraphPredicatePatchRequest {
	/// Tenant to query within.
	pub tenant_id: String,
	/// Project to query within.
	pub project_id: String,
	/// Agent requesting the mutation.
	pub agent_id: String,
	/// Optional auth token identifier used for super-admin checks.
	pub token_id: Option<String>,
	/// Predicate identifier to mutate.
	pub predicate_id: Uuid,
	/// Optional new predicate status.
	pub status: Option<String>,
	/// Optional new cardinality value.
	pub cardinality: Option<String>,
}

/// Request payload for adding a graph predicate alias.
#[derive(Clone, Debug)]
pub struct AdminGraphPredicateAliasAddRequest {
	/// Tenant to query within.
	pub tenant_id: String,
	/// Project to query within.
	pub project_id: String,
	/// Agent requesting the mutation.
	pub agent_id: String,
	/// Optional auth token identifier used for super-admin checks.
	pub token_id: Option<String>,
	/// Predicate identifier to extend.
	pub predicate_id: Uuid,
	/// Alias surface to add.
	pub alias: String,
}

/// Request payload for listing graph predicate aliases.
#[derive(Clone, Debug)]
pub struct AdminGraphPredicateAliasesListRequest {
	/// Tenant to query within.
	pub tenant_id: String,
	/// Project to query within.
	pub project_id: String,
	/// Agent requesting the list.
	pub agent_id: String,
	/// Predicate identifier to inspect.
	pub predicate_id: Uuid,
}

/// Serialized graph predicate returned by admin APIs.
#[derive(Clone, Debug, Serialize)]
pub struct AdminGraphPredicateResponse {
	/// Predicate identifier.
	pub predicate_id: Uuid,
	/// Predicate scope key.
	pub scope_key: String,
	/// Tenant scope when tenant-specific.
	pub tenant_id: Option<String>,
	/// Project scope when project-specific.
	pub project_id: Option<String>,
	/// Canonical predicate surface.
	pub canonical: String,
	/// Normalized canonical predicate surface.
	pub canonical_norm: String,
	/// Cardinality policy.
	pub cardinality: String,
	/// Lifecycle status.
	pub status: String,
	#[serde(with = "crate::time_serde")]
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Serialized graph predicate alias returned by admin APIs.
#[derive(Clone, Debug, Serialize)]
pub struct AdminGraphPredicateAliasResponse {
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
	#[serde(with = "crate::time_serde")]
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Response payload for listing graph predicates.
#[derive(Clone, Debug, Serialize)]
pub struct AdminGraphPredicatesListResponse {
	/// Returned predicates.
	pub predicates: Vec<AdminGraphPredicateResponse>,
}

/// Response payload for graph predicate alias operations.
#[derive(Clone, Debug, Serialize)]
pub struct AdminGraphPredicateAliasesResponse {
	/// Predicate identifier.
	pub predicate_id: Uuid,
	/// Returned aliases.
	pub aliases: Vec<AdminGraphPredicateAliasResponse>,
}
