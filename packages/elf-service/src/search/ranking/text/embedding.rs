pub(crate) fn build_dense_embedding_input(
	query: &str,
	project_context_description: Option<&str>,
) -> String {
	let Some(description) = project_context_description else { return query.to_string() };
	let trimmed = description.trim();

	if trimmed.is_empty() {
		return query.to_string();
	}

	format!("{query}\n\nProject context:\n{trimmed}")
}
