#[derive(Default)]
pub(in crate::search::filter) struct FilterParseState {
	pub(in crate::search::filter) nodes: usize,
	pub(in crate::search::filter) max_depth: usize,
}
