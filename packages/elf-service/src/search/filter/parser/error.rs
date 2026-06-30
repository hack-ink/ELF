#[derive(Clone, Debug)]
pub(crate) struct FilterParseError {
	pub(in crate::search::filter) path: String,
	pub(in crate::search::filter) message: String,
}
impl std::fmt::Display for FilterParseError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}: {}", self.path, self.message)
	}
}
