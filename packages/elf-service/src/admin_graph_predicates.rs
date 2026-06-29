//! Administrative graph-predicate APIs.

mod helpers;
mod service;
mod types;

pub use types::{
	AdminGraphPredicateAliasAddRequest, AdminGraphPredicateAliasResponse,
	AdminGraphPredicateAliasesListRequest, AdminGraphPredicateAliasesResponse,
	AdminGraphPredicatePatchRequest, AdminGraphPredicateResponse, AdminGraphPredicatesListRequest,
	AdminGraphPredicatesListResponse,
};
