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
				"tables/016_graph_entities.sql" =>
					out.push_str(include_str!("../../../sql/tables/016_graph_entities.sql")),
				"tables/017_graph_entity_aliases.sql" =>
					out.push_str(include_str!("../../../sql/tables/017_graph_entity_aliases.sql")),
				"tables/020_graph_predicates.sql" =>
					out.push_str(include_str!("../../../sql/tables/020_graph_predicates.sql")),
				"tables/021_graph_predicate_aliases.sql" => out
					.push_str(include_str!("../../../sql/tables/021_graph_predicate_aliases.sql")),
				"tables/018_graph_facts.sql" =>
					out.push_str(include_str!("../../../sql/tables/018_graph_facts.sql")),
				"tables/019_graph_fact_evidence.sql" =>
					out.push_str(include_str!("../../../sql/tables/019_graph_fact_evidence.sql")),
				"tables/022_graph_fact_supersessions.sql" => out
					.push_str(include_str!("../../../sql/tables/022_graph_fact_supersessions.sql")),
				"tables/013_memory_note_fields.sql" =>
					out.push_str(include_str!("../../../sql/tables/013_memory_note_fields.sql")),
				"tables/009_memory_note_chunks.sql" =>
					out.push_str(include_str!("../../../sql/tables/009_memory_note_chunks.sql")),
				"tables/010_note_chunk_embeddings.sql" =>
					out.push_str(include_str!("../../../sql/tables/010_note_chunk_embeddings.sql")),
				"tables/014_note_field_embeddings.sql" =>
					out.push_str(include_str!("../../../sql/tables/014_note_field_embeddings.sql")),
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
				"tables/012_search_trace_candidates.sql" => out
					.push_str(include_str!("../../../sql/tables/012_search_trace_candidates.sql")),
				"tables/015_search_trace_stages.sql" =>
					out.push_str(include_str!("../../../sql/tables/015_search_trace_stages.sql")),
				"tables/007_search_trace_outbox.sql" =>
					out.push_str(include_str!("../../../sql/tables/007_search_trace_outbox.sql")),
				"tables/008_llm_cache.sql" =>
					out.push_str(include_str!("../../../sql/tables/008_llm_cache.sql")),
				"tables/011_search_sessions.sql" =>
					out.push_str(include_str!("../../../sql/tables/011_search_sessions.sql")),
				"tables/025_doc_documents.sql" =>
					out.push_str(include_str!("../../../sql/tables/025_doc_documents.sql")),
				"tables/026_doc_chunks.sql" =>
					out.push_str(include_str!("../../../sql/tables/026_doc_chunks.sql")),
				"tables/027_doc_chunk_embeddings.sql" =>
					out.push_str(include_str!("../../../sql/tables/027_doc_chunk_embeddings.sql")),
				"tables/028_doc_indexing_outbox.sql" =>
					out.push_str(include_str!("../../../sql/tables/028_doc_indexing_outbox.sql")),
				"tables/023_memory_ingest_decisions.sql" => out
					.push_str(include_str!("../../../sql/tables/023_memory_ingest_decisions.sql")),
				"tables/024_memory_space_grants.sql" =>
					out.push_str(include_str!("../../../sql/tables/024_memory_space_grants.sql")),
				_ => out.push_str(line),
			}
		} else {
			out.push_str(line);
		}

		out.push('\n');
	}

	out
}
