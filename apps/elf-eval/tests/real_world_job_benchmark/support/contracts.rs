pub(crate) struct RecallDebugSourceContract<'a> {
	pub(crate) service: &'a str,
	pub(crate) service_lib: &'a str,
	pub(crate) routes: &'a str,
	pub(crate) mcp: &'a str,
	pub(crate) recall_spec: &'a str,
	pub(crate) service_spec: &'a str,
	pub(crate) version_registry: &'a str,
	pub(crate) markdown: &'a str,
	pub(crate) benchmarking_index: &'a str,
	pub(crate) readme: &'a str,
}
