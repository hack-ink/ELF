#![allow(clippy::single_component_path_imports, unused_crate_dependencies)]

//! Offline adapter for agentmemory-style fixture exports.

use std::{collections::HashMap, fs, path::PathBuf};

use clap::Parser;
use color_eyre;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

const OUTPUT_SCHEMA: &str = "elf.agentmemory_adapter/v1";
const FIXTURE_RESOLVER: &str = "agentmemory_fixture/v1";
const DEFAULT_IMPORTANCE: f32 = 0.5;
const DEFAULT_CONFIDENCE: f32 = 0.5;

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
struct Args {
	/// Path to a sanitized agentmemory-style JSON fixture.
	#[arg(long, short = 'f', value_name = "FILE")]
	fixture: PathBuf,
	/// Write adapter JSON to this file (defaults to stdout).
	#[arg(long, value_name = "FILE")]
	out: Option<PathBuf>,
	/// ELF write scope to attach to emitted note and doc candidates.
	#[arg(long, default_value = "agent_private")]
	scope: String,
	/// Maximum note text length accepted for note candidates.
	#[arg(long, default_value_t = 240)]
	max_note_chars: usize,
}

#[derive(Debug, Deserialize)]
struct AgentmemoryFixture {
	schema: Option<String>,

	fixture_id: Option<String>,
	#[serde(default)]
	source: FixtureSource,
	#[serde(default)]
	sessions: Vec<AgentmemorySession>,
}

#[derive(Debug, Default, Deserialize)]
struct FixtureSource {
	system: Option<String>,

	version: Option<String>,

	export_id: Option<String>,

	exported_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgentmemorySession {
	session_id: String,

	agent: Option<String>,

	project: Option<String>,

	started_at: Option<String>,

	ended_at: Option<String>,
	#[serde(default)]
	observations: Vec<AgentmemoryObservation>,
	#[serde(default)]
	memories: Vec<AgentmemoryMemory>,
	#[serde(default)]
	retrieval_cases: Vec<AgentmemoryRetrievalCase>,
}

#[derive(Debug, Deserialize)]
struct AgentmemoryObservation {
	observation_id: String,

	ts: Option<String>,

	role: Option<String>,

	kind: Option<String>,
	text: String,
	#[serde(default)]
	metadata: Value,
}

#[derive(Debug, Deserialize)]
struct AgentmemoryMemory {
	memory_id: String,

	kind: Option<String>,

	key: Option<String>,
	text: String,

	importance: Option<f32>,

	confidence: Option<f32>,

	ttl_days: Option<i64>,

	created_at: Option<String>,

	updated_at: Option<String>,
	#[serde(default)]
	source_observation_ids: Vec<String>,
	#[serde(default)]
	metadata: Value,
}

#[derive(Debug, Deserialize)]
struct AgentmemoryRetrievalCase {
	query_id: String,
	query: String,
	#[serde(default)]
	expected_memory_ids: Vec<String>,
	#[serde(default)]
	agentmemory_results: Vec<AgentmemorySearchResult>,
	#[serde(default)]
	metadata: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AgentmemorySearchResult {
	memory_id: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	rank: Option<u32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	score: Option<f32>,
}

#[derive(Debug, Serialize)]
struct AdapterOutput {
	schema: &'static str,
	fixture_id: String,
	source: AdapterSource,
	summary: AdapterSummary,
	note_candidates: Vec<NoteCandidate>,
	doc_candidates: Vec<DocCandidate>,
	baseline_queries: Vec<BaselineQuery>,
	ignored_items: Vec<IgnoredItem>,
}

#[derive(Debug, Serialize)]
struct AdapterSource {
	system: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	version: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	export_id: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	exported_at: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	fixture_schema: Option<String>,
}

#[derive(Debug, Serialize)]
struct AdapterSummary {
	session_count: usize,
	observation_count: usize,
	memory_count: usize,
	note_candidate_count: usize,
	doc_candidate_count: usize,
	baseline_query_count: usize,
	ignored_count: usize,
}

#[derive(Clone, Debug, Serialize)]
struct NoteCandidate {
	candidate_id: Uuid,
	scope: String,
	session_id: String,
	source_memory_id: String,
	source_observation_ids: Vec<String>,
	notes_ingest_item: ElfNoteCandidate,
	#[serde(skip_serializing_if = "Value::is_null")]
	source_metadata: Value,
}

#[derive(Clone, Debug, Serialize)]
struct ElfNoteCandidate {
	#[serde(rename = "type")]
	note_type: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	key: Option<String>,
	text: String,
	importance: f32,
	confidence: f32,
	#[serde(skip_serializing_if = "Option::is_none")]
	ttl_days: Option<i64>,
	source_ref: Value,
}

#[derive(Debug, Serialize)]
struct DocCandidate {
	candidate_id: Uuid,
	scope: String,
	session_id: String,
	source_observation_id: String,
	docs_put: DocsPutCandidate,
	#[serde(skip_serializing_if = "Value::is_null")]
	source_metadata: Value,
}

#[derive(Debug, Serialize)]
struct DocsPutCandidate {
	scope: String,
	doc_type: &'static str,
	title: String,
	source_ref: Value,
	content: String,
}

#[derive(Debug, Serialize)]
struct BaselineQuery {
	query_id: String,
	session_id: String,
	query: String,
	expected_source_memory_ids: Vec<String>,
	expected_candidate_ids: Vec<Uuid>,
	expected_keys: Vec<String>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	agentmemory_results: Vec<AgentmemorySearchResult>,
	#[serde(skip_serializing_if = "Value::is_null")]
	source_metadata: Value,
}

#[derive(Debug, Serialize)]
struct IgnoredItem {
	item_kind: &'static str,
	session_id: String,
	source_id: String,
	reason: &'static str,
	#[serde(skip_serializing_if = "Option::is_none")]
	detail: Option<String>,
}

#[derive(Clone)]
struct FixtureContext {
	fixture_id: String,
	source_system: String,
	source_version: Option<String>,
	exported_at: Option<String>,
	scope: String,
	max_note_chars: usize,
}

fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;

	let args = Args::parse();
	let raw = fs::read_to_string(&args.fixture)?;
	let fixture: AgentmemoryFixture = serde_json::from_str(&raw)?;
	let output = adapt_fixture(&fixture, args.scope.as_str(), args.max_note_chars);
	let json = serde_json::to_string_pretty(&output)?;

	if let Some(path) = args.out {
		write_output(path, json.as_str())?;
	} else {
		println!("{json}");
	}

	Ok(())
}

fn write_output(path: PathBuf, json: &str) -> color_eyre::Result<()> {
	if let Some(parent) = path.parent()
		&& !parent.as_os_str().is_empty()
	{
		fs::create_dir_all(parent)?;
	}

	fs::write(path, json)?;

	Ok(())
}

fn adapt_fixture(
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
		system: clean_string(fixture.source.system.as_deref())
			.unwrap_or_else(|| "agentmemory".to_string()),
		version: clean_string(fixture.source.version.as_deref()),
		export_id: clean_string(fixture.source.export_id.as_deref()),
		exported_at: clean_string(fixture.source.exported_at.as_deref()),
		fixture_schema: clean_string(fixture.schema.as_deref()),
	}
}

fn fixture_id(fixture: &AgentmemoryFixture, source_system: &str) -> String {
	clean_string(fixture.fixture_id.as_deref())
		.or_else(|| clean_string(fixture.source.export_id.as_deref()))
		.unwrap_or_else(|| stable_uuid("fixture", &[source_system]).to_string())
}

fn map_observations(
	session: &AgentmemorySession,
	ctx: &FixtureContext,
	docs: &mut Vec<DocCandidate>,
	ignored: &mut Vec<IgnoredItem>,
) {
	for observation in &session.observations {
		match doc_candidate(session, observation, ctx) {
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
		match note_candidate(session, memory, ctx) {
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
		match baseline_query(session, case, memory_map) {
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

fn doc_candidate(
	session: &AgentmemorySession,
	observation: &AgentmemoryObservation,
	ctx: &FixtureContext,
) -> std::result::Result<DocCandidate, &'static str> {
	let text = observation.text.trim();

	if text.is_empty() {
		return Err("empty_text");
	}

	let Some(ts) = observation_timestamp(session, observation, ctx) else {
		return Err("missing_or_invalid_timestamp");
	};
	let candidate_id = stable_uuid(
		"observation",
		&[
			ctx.fixture_id.as_str(),
			session.session_id.as_str(),
			observation.observation_id.as_str(),
		],
	);
	let role = clean_string(observation.role.as_deref())
		.or_else(|| clean_string(observation.kind.as_deref()))
		.unwrap_or_else(|| "observation".to_string());
	let title = format!("agentmemory observation {}", observation.observation_id);
	let source_ref = serde_json::json!({
		"schema": "doc_source_ref/v1",
		"doc_type": "chat",
		"ts": ts,
		"thread_id": session.session_id,
		"role": role,
		"message_id": observation.observation_id,
		"agentmemory_fixture_id": ctx.fixture_id,
		"agentmemory_source_system": ctx.source_system,
		"agentmemory_observation_kind": clean_string(observation.kind.as_deref()),
		"agent": clean_string(session.agent.as_deref()),
		"project": clean_string(session.project.as_deref()),
	});

	Ok(DocCandidate {
		candidate_id,
		scope: ctx.scope.clone(),
		session_id: session.session_id.clone(),
		source_observation_id: observation.observation_id.clone(),
		docs_put: DocsPutCandidate {
			scope: ctx.scope.clone(),
			doc_type: "chat",
			title,
			source_ref,
			content: observation.text.clone(),
		},
		source_metadata: observation.metadata.clone(),
	})
}

fn note_candidate(
	session: &AgentmemorySession,
	memory: &AgentmemoryMemory,
	ctx: &FixtureContext,
) -> std::result::Result<NoteCandidate, &'static str> {
	let text = memory.text.trim();

	if text.is_empty() {
		return Err("empty_text");
	}
	if text.chars().count() > ctx.max_note_chars {
		return Err("note_text_too_long");
	}

	let Some(note_type) = memory.kind.as_deref().and_then(map_note_type) else {
		return Err("unsupported_memory_kind");
	};
	let Some(importance) = score_or_default(memory.importance, DEFAULT_IMPORTANCE) else {
		return Err("invalid_importance");
	};
	let Some(confidence) = score_or_default(memory.confidence, DEFAULT_CONFIDENCE) else {
		return Err("invalid_confidence");
	};
	let candidate_id = stable_uuid(
		"memory",
		&[ctx.fixture_id.as_str(), session.session_id.as_str(), memory.memory_id.as_str()],
	);
	let source_ref = note_source_ref(session, memory, ctx);

	Ok(NoteCandidate {
		candidate_id,
		scope: ctx.scope.clone(),
		session_id: session.session_id.clone(),
		source_memory_id: memory.memory_id.clone(),
		source_observation_ids: memory.source_observation_ids.clone(),
		notes_ingest_item: ElfNoteCandidate {
			note_type: note_type.to_string(),
			key: clean_string(memory.key.as_deref()),
			text: memory.text.clone(),
			importance,
			confidence,
			ttl_days: memory.ttl_days.filter(|days| *days > 0),
			source_ref,
		},
		source_metadata: memory.metadata.clone(),
	})
}

fn note_source_ref(
	session: &AgentmemorySession,
	memory: &AgentmemoryMemory,
	ctx: &FixtureContext,
) -> Value {
	serde_json::json!({
		"schema": "source_ref/v1",
		"resolver": FIXTURE_RESOLVER,
		"ref": {
			"fixture_id": ctx.fixture_id,
			"session_id": session.session_id,
			"memory_id": memory.memory_id,
			"observation_ids": memory.source_observation_ids,
		},
		"state": {
			"source_system": ctx.source_system,
			"source_version": ctx.source_version,
			"exported_at": ctx.exported_at,
			"session_started_at": session.started_at,
			"session_ended_at": session.ended_at,
			"memory_created_at": memory.created_at,
			"memory_updated_at": memory.updated_at,
		},
		"locator": {
			"memory_id": memory.memory_id,
			"observation_ids": memory.source_observation_ids,
		},
		"hints": {
			"agent": session.agent,
			"project": session.project,
			"origin_kind": memory.kind,
		},
	})
}

fn baseline_query(
	session: &AgentmemorySession,
	case: &AgentmemoryRetrievalCase,
	memory_map: &HashMap<String, NoteCandidate>,
) -> Option<BaselineQuery> {
	if case.query.trim().is_empty() || case.expected_memory_ids.is_empty() {
		return None;
	}

	let expected: Vec<&NoteCandidate> =
		case.expected_memory_ids.iter().filter_map(|id| memory_map.get(id)).collect();

	if expected.is_empty() {
		return None;
	}

	Some(BaselineQuery {
		query_id: case.query_id.clone(),
		session_id: session.session_id.clone(),
		query: case.query.clone(),
		expected_source_memory_ids: expected
			.iter()
			.map(|candidate| candidate.source_memory_id.clone())
			.collect(),
		expected_candidate_ids: expected.iter().map(|candidate| candidate.candidate_id).collect(),
		expected_keys: expected
			.iter()
			.filter_map(|candidate| candidate.notes_ingest_item.key.clone())
			.collect(),
		agentmemory_results: case.agentmemory_results.clone(),
		source_metadata: case.metadata.clone(),
	})
}

fn observation_timestamp(
	session: &AgentmemorySession,
	observation: &AgentmemoryObservation,
	ctx: &FixtureContext,
) -> Option<String> {
	[observation.ts.as_deref(), session.started_at.as_deref(), ctx.exported_at.as_deref()]
		.into_iter()
		.flatten()
		.find_map(normalize_rfc3339)
}

fn normalize_rfc3339(value: &str) -> Option<String> {
	OffsetDateTime::parse(value, &Rfc3339)
		.ok()
		.and_then(|timestamp| timestamp.format(&Rfc3339).ok())
}

fn map_note_type(kind: &str) -> Option<&'static str> {
	match kind.trim().to_ascii_lowercase().as_str() {
		"preference" => Some("preference"),
		"constraint" => Some("constraint"),
		"decision" => Some("decision"),
		"profile" => Some("profile"),
		"fact" => Some("fact"),
		"plan" => Some("plan"),
		_ => None,
	}
}

fn score_or_default(score: Option<f32>, default: f32) -> Option<f32> {
	let score = score.unwrap_or(default);

	if score.is_finite() && (0.0..=1.0).contains(&score) { Some(score) } else { None }
}

fn clean_string(value: Option<&str>) -> Option<String> {
	value.map(str::trim).filter(|value| !value.is_empty()).map(str::to_string)
}

fn stable_uuid(kind: &str, parts: &[&str]) -> Uuid {
	let mut key = format!("https://hack.ink/elf/{OUTPUT_SCHEMA}/{kind}");

	for part in parts {
		key.push('/');
		key.push_str(part);
	}

	Uuid::new_v5(&Uuid::NAMESPACE_URL, key.as_bytes())
}
