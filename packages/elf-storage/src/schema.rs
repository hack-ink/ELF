pub fn render_schema(vector_dim: u32) -> String {
	let init = include_str!("../../../sql/init.sql");
	let expanded = expand_includes(init);
	expanded.replace("<VECTOR_DIM>", &vector_dim.to_string())
}

fn expand_includes(sql: &str) -> String {
	let mut out = String::new();
	for line in sql.lines() {
		let trimmed = line.trim();
		if let Some(path) = trimmed.strip_prefix("\\ir ") {
			match path.trim() {
				"00_extensions.sql" => out.push_str(include_str!("../../../sql/00_extensions.sql")),
				"tables/001_memory_notes.sql" =>
					out.push_str(include_str!("../../../sql/tables/001_memory_notes.sql")),
				"tables/002_note_embeddings.sql" =>
					out.push_str(include_str!("../../../sql/tables/002_note_embeddings.sql")),
				"tables/003_memory_note_versions.sql" =>
					out.push_str(include_str!("../../../sql/tables/003_memory_note_versions.sql")),
				"tables/004_memory_hits.sql" =>
					out.push_str(include_str!("../../../sql/tables/004_memory_hits.sql")),
				"tables/005_indexing_outbox.sql" =>
					out.push_str(include_str!("../../../sql/tables/005_indexing_outbox.sql")),
				"tables/006_search_traces.sql" =>
					out.push_str(include_str!("../../../sql/tables/006_search_traces.sql")),
				"tables/007_search_trace_outbox.sql" =>
					out.push_str(include_str!("../../../sql/tables/007_search_trace_outbox.sql")),
				_ => out.push_str(line),
			}
		} else {
			out.push_str(line);
		}
		out.push('\n');
	}
	out
}
