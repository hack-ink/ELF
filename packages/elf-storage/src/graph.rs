//! Graph entity, predicate, and fact storage helpers.

mod entity;
mod fact;
mod predicate;

pub use self::{
	entity::{resolve_entity_by_surface, upsert_entity, upsert_entity_alias},
	fact::{
		fetch_active_facts_for_subject, insert_fact_with_evidence,
		supersede_conflicting_active_facts, upsert_fact_with_evidence,
	},
	predicate::{
		add_predicate_alias, get_predicate_by_id, list_predicate_aliases,
		list_predicates_by_scope_keys, resolve_or_register_predicate,
		resolve_predicate_no_register, update_predicate, update_predicate_guarded,
	},
};

use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	Error, Result,
	models::{GraphEntity, GraphFact},
};

const GRAPH_PREDICATE_SCOPE_GLOBAL: &str = "__global__";
const GRAPH_PREDICATE_SCOPE_PROJECT_PREFIX: &str = "__project__:";

/// Normalizes graph entity surfaces for uniqueness and lookup.
pub fn normalize_entity_name(input: &str) -> String {
	input.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Normalizes graph predicate surfaces for uniqueness and lookup.
pub fn normalize_predicate_name(input: &str) -> String {
	normalize_entity_name(input)
}

fn predicate_scope_key_tenant_project(tenant_id: &str, project_id: &str) -> String {
	format!("{tenant_id}:{project_id}")
}

fn predicate_scope_key_project(project_id: &str) -> String {
	format!("{GRAPH_PREDICATE_SCOPE_PROJECT_PREFIX}{project_id}")
}
