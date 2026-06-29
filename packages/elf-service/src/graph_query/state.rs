use super::*;

#[derive(Debug)]
pub(super) struct PreparedGraphQuery {
	pub(super) tenant_id: String,
	pub(super) project_id: String,
	pub(super) agent_id: String,
	pub(super) read_profile: String,
	pub(super) subject: GraphQueryEntityRef,
	pub(super) predicate: Option<GraphQueryPredicateRef>,
	pub(super) requested_scopes: Vec<String>,
	pub(super) as_of: OffsetDateTime,
	pub(super) limit: usize,
	pub(super) explain: bool,
}

#[derive(Debug)]
pub(super) struct ResolvedGraphQuerySubject {
	pub(super) entity_id: Uuid,
	pub(super) canonical: String,
	pub(super) kind: Option<String>,
}

#[derive(Debug)]
pub(super) struct ResolvedGraphQueryPredicate {
	pub(super) id: Uuid,
	pub(super) canonical: String,
}

#[derive(Debug)]
pub(super) struct GraphQueryRowsFetchParams<'a> {
	pub(super) tenant_id: &'a str,
	pub(super) project_id: &'a str,
	pub(super) subject_entity_id: Uuid,
	pub(super) scopes: &'a [String],
	pub(super) as_of: OffsetDateTime,
	pub(super) actor: &'a str,
	pub(super) shared_scope_keys: &'a [String],
	pub(super) predicate_id: Option<Uuid>,
	pub(super) limit_plus_one: i64,
}

#[derive(Debug, FromRow)]
pub(super) struct GraphQueryFactRow {
	pub(super) fact_id: Uuid,
	pub(super) scope: String,
	pub(super) actor: String,
	pub(super) predicate: String,
	pub(super) predicate_id: Option<Uuid>,
	pub(super) object_entity_id: Option<Uuid>,
	pub(super) object_canonical: Option<String>,
	pub(super) object_kind: Option<String>,
	pub(super) object_value: Option<String>,
	pub(super) valid_from: OffsetDateTime,
	pub(super) valid_to: Option<OffsetDateTime>,
	pub(super) evidence_note_ids: Vec<Uuid>,
}
