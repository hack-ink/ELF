use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub(super) struct AgentmemoryFixture {
	pub(super) schema: Option<String>,

	pub(super) fixture_id: Option<String>,
	#[serde(default)]
	pub(super) source: FixtureSource,
	#[serde(default)]
	pub(super) sessions: Vec<AgentmemorySession>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct FixtureSource {
	pub(super) system: Option<String>,

	pub(super) version: Option<String>,

	pub(super) export_id: Option<String>,

	pub(super) exported_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AgentmemorySession {
	pub(super) session_id: String,

	pub(super) agent: Option<String>,

	pub(super) project: Option<String>,

	pub(super) started_at: Option<String>,

	pub(super) ended_at: Option<String>,
	#[serde(default)]
	pub(super) observations: Vec<AgentmemoryObservation>,
	#[serde(default)]
	pub(super) memories: Vec<AgentmemoryMemory>,
	#[serde(default)]
	pub(super) retrieval_cases: Vec<AgentmemoryRetrievalCase>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AgentmemoryObservation {
	pub(super) observation_id: String,

	pub(super) ts: Option<String>,

	pub(super) role: Option<String>,

	pub(super) kind: Option<String>,
	pub(super) text: String,
	#[serde(default)]
	pub(super) metadata: Value,
}

#[derive(Debug, Deserialize)]
pub(super) struct AgentmemoryMemory {
	pub(super) memory_id: String,

	pub(super) kind: Option<String>,

	pub(super) key: Option<String>,
	pub(super) text: String,

	pub(super) importance: Option<f32>,

	pub(super) confidence: Option<f32>,

	pub(super) ttl_days: Option<i64>,

	pub(super) created_at: Option<String>,

	pub(super) updated_at: Option<String>,
	#[serde(default)]
	pub(super) source_observation_ids: Vec<String>,
	#[serde(default)]
	pub(super) metadata: Value,
}

#[derive(Debug, Deserialize)]
pub(super) struct AgentmemoryRetrievalCase {
	pub(super) query_id: String,
	pub(super) query: String,
	#[serde(default)]
	pub(super) expected_memory_ids: Vec<String>,
	#[serde(default)]
	pub(super) agentmemory_results: Vec<AgentmemorySearchResult>,
	#[serde(default)]
	pub(super) metadata: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct AgentmemorySearchResult {
	pub(super) memory_id: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) rank: Option<u32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) score: Option<f32>,
}

#[derive(Debug, Serialize)]
pub(super) struct AdapterOutput {
	pub(super) schema: &'static str,
	pub(super) fixture_id: String,
	pub(super) source: AdapterSource,
	pub(super) summary: AdapterSummary,
	pub(super) note_candidates: Vec<NoteCandidate>,
	pub(super) doc_candidates: Vec<DocCandidate>,
	pub(super) baseline_queries: Vec<BaselineQuery>,
	pub(super) ignored_items: Vec<IgnoredItem>,
}

#[derive(Debug, Serialize)]
pub(super) struct AdapterSource {
	pub(super) system: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) version: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) export_id: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) exported_at: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) fixture_schema: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct AdapterSummary {
	pub(super) session_count: usize,
	pub(super) observation_count: usize,
	pub(super) memory_count: usize,
	pub(super) note_candidate_count: usize,
	pub(super) doc_candidate_count: usize,
	pub(super) baseline_query_count: usize,
	pub(super) ignored_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct NoteCandidate {
	pub(super) candidate_id: Uuid,
	pub(super) scope: String,
	pub(super) session_id: String,
	pub(super) source_memory_id: String,
	pub(super) source_observation_ids: Vec<String>,
	pub(super) notes_ingest_item: ElfNoteCandidate,
	#[serde(skip_serializing_if = "Value::is_null")]
	pub(super) source_metadata: Value,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct ElfNoteCandidate {
	#[serde(rename = "type")]
	pub(super) note_type: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) key: Option<String>,
	pub(super) text: String,
	pub(super) importance: f32,
	pub(super) confidence: f32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) ttl_days: Option<i64>,
	pub(super) source_ref: Value,
}

#[derive(Debug, Serialize)]
pub(super) struct DocCandidate {
	pub(super) candidate_id: Uuid,
	pub(super) scope: String,
	pub(super) session_id: String,
	pub(super) source_observation_id: String,
	pub(super) docs_put: DocsPutCandidate,
	#[serde(skip_serializing_if = "Value::is_null")]
	pub(super) source_metadata: Value,
}

#[derive(Debug, Serialize)]
pub(super) struct DocsPutCandidate {
	pub(super) scope: String,
	pub(super) doc_type: &'static str,
	pub(super) title: String,
	pub(super) source_ref: Value,
	pub(super) content: String,
}

#[derive(Debug, Serialize)]
pub(super) struct BaselineQuery {
	pub(super) query_id: String,
	pub(super) session_id: String,
	pub(super) query: String,
	pub(super) expected_source_memory_ids: Vec<String>,
	pub(super) expected_candidate_ids: Vec<Uuid>,
	pub(super) expected_keys: Vec<String>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub(super) agentmemory_results: Vec<AgentmemorySearchResult>,
	#[serde(skip_serializing_if = "Value::is_null")]
	pub(super) source_metadata: Value,
}

#[derive(Debug, Serialize)]
pub(super) struct IgnoredItem {
	pub(super) item_kind: &'static str,
	pub(super) session_id: String,
	pub(super) source_id: String,
	pub(super) reason: &'static str,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) detail: Option<String>,
}

#[derive(Clone)]
pub(super) struct FixtureContext {
	pub(super) fixture_id: String,
	pub(super) source_system: String,
	pub(super) source_version: Option<String>,
	pub(super) exported_at: Option<String>,
	pub(super) scope: String,
	pub(super) max_note_chars: usize,
}
