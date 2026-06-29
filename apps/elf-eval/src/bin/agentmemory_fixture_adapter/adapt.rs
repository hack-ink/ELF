use std::collections::HashMap;

use crate::{
	OUTPUT_SCHEMA,
	mapping::{self},
	types::{
		AdapterOutput, AdapterSource, AdapterSummary, AgentmemoryFixture, AgentmemorySession,
		BaselineQuery, DocCandidate, FixtureContext, IgnoredItem, NoteCandidate,
	},
	util::{self},
};

pub(super) fn adapt_fixture(
	fixture: &AgentmemoryFixture,
	scope: &str,
	max_note_chars: usize,
) -> AdapterOutput {
	let source = adapter_source(fixture);
	let fixture_id = fixture_id(fixture, source.system.as_str());
	let ctx = FixtureContext {
		fixture_id: fixture_id.clone(),
		source_system: source.system.clone(),
		source_version: source.version.clone(),
		exported_at: source.exported_at.clone(),
		scope: scope.to_string(),
		max_note_chars,
	};
	let mut notes = Vec::new();
	let mut docs = Vec::new();
	let mut baselines = Vec::new();
	let mut ignored = Vec::new();
	let mut memory_map = HashMap::new();

	for session in &fixture.sessions {
		map_observations(session, &ctx, &mut docs, &mut ignored);
		map_memories(session, &ctx, &mut notes, &mut memory_map, &mut ignored);
		map_baselines(session, &memory_map, &mut baselines, &mut ignored);
	}

	AdapterOutput {
		schema: OUTPUT_SCHEMA,
		fixture_id,
		source,
		summary: AdapterSummary {
			session_count: fixture.sessions.len(),
			observation_count: fixture
				.sessions
				.iter()
				.map(|session| session.observations.len())
				.sum(),
			memory_count: fixture.sessions.iter().map(|session| session.memories.len()).sum(),
			note_candidate_count: notes.len(),
			doc_candidate_count: docs.len(),
			baseline_query_count: baselines.len(),
			ignored_count: ignored.len(),
		},
		note_candidates: notes,
		doc_candidates: docs,
		baseline_queries: baselines,
		ignored_items: ignored,
	}
}

fn adapter_source(fixture: &AgentmemoryFixture) -> AdapterSource {
	AdapterSource {
		system: util::clean_string(fixture.source.system.as_deref())
			.unwrap_or_else(|| "agentmemory".to_string()),
		version: util::clean_string(fixture.source.version.as_deref()),
		export_id: util::clean_string(fixture.source.export_id.as_deref()),
		exported_at: util::clean_string(fixture.source.exported_at.as_deref()),
		fixture_schema: util::clean_string(fixture.schema.as_deref()),
	}
}

fn fixture_id(fixture: &AgentmemoryFixture, source_system: &str) -> String {
	util::clean_string(fixture.fixture_id.as_deref())
		.or_else(|| util::clean_string(fixture.source.export_id.as_deref()))
		.unwrap_or_else(|| util::stable_uuid("fixture", &[source_system]).to_string())
}

fn map_observations(
	session: &AgentmemorySession,
	ctx: &FixtureContext,
	docs: &mut Vec<DocCandidate>,
	ignored: &mut Vec<IgnoredItem>,
) {
	for observation in &session.observations {
		match mapping::doc_candidate(session, observation, ctx) {
			Ok(candidate) => docs.push(candidate),
			Err(reason) => ignored.push(IgnoredItem {
				item_kind: "observation",
				session_id: session.session_id.clone(),
				source_id: observation.observation_id.clone(),
				reason,
				detail: None,
			}),
		}
	}
}

fn map_memories(
	session: &AgentmemorySession,
	ctx: &FixtureContext,
	notes: &mut Vec<NoteCandidate>,
	memory_map: &mut HashMap<String, NoteCandidate>,
	ignored: &mut Vec<IgnoredItem>,
) {
	for memory in &session.memories {
		match mapping::note_candidate(session, memory, ctx) {
			Ok(candidate) => {
				memory_map.insert(memory.memory_id.clone(), candidate.clone());
				notes.push(candidate);
			},
			Err(reason) => ignored.push(IgnoredItem {
				item_kind: "memory",
				session_id: session.session_id.clone(),
				source_id: memory.memory_id.clone(),
				reason,
				detail: None,
			}),
		}
	}
}

fn map_baselines(
	session: &AgentmemorySession,
	memory_map: &HashMap<String, NoteCandidate>,
	baselines: &mut Vec<BaselineQuery>,
	ignored: &mut Vec<IgnoredItem>,
) {
	for case in &session.retrieval_cases {
		match mapping::baseline_query(session, case, memory_map) {
			Some(baseline) => baselines.push(baseline),
			None => ignored.push(IgnoredItem {
				item_kind: "retrieval_case",
				session_id: session.session_id.clone(),
				source_id: case.query_id.clone(),
				reason: "no_mapped_expected_memories",
				detail: None,
			}),
		}
	}
}
