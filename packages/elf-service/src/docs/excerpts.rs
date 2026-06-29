mod access;
mod filter;
mod locator;
mod match_resolution;
mod text;

pub(super) use self::{
	access::{doc_read_allowed, parse_scored_point_uuid_id},
	filter::build_doc_search_filter,
	locator::{build_docs_l0_pointer, docs_excerpt_locator},
	match_resolution::{docs_excerpts_resolve_windowed_match, load_docs_excerpt_context},
};
#[cfg(test)] pub(super) use text::should_enable_sparse_auto;
pub(super) use text::{docs_search_sparse_enabled, truncate_bytes};
