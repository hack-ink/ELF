mod aliases;
mod query;
mod resolution;
mod update;

pub use self::{
	aliases::{add_predicate_alias, list_predicate_aliases},
	query::{get_predicate_by_id, list_predicates_by_scope_keys},
	resolution::{resolve_or_register_predicate, resolve_predicate_no_register},
	update::{update_predicate, update_predicate_guarded},
};
