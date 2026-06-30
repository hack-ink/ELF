use crate::{Deserialize, Serialize, Uuid};

#[derive(Debug, Deserialize)]
pub(crate) struct QueryManifest {
	pub(crate) queries: Vec<QueryCase>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct QueryCase {
	pub(crate) id: String,
	pub(crate) task: Option<String>,
	pub(crate) query: String,
	pub(crate) expected_doc: String,
	pub(crate) expected_terms: Vec<String>,
	#[serde(default)]
	pub(crate) allowed_alternate_docs: Vec<String>,
	#[serde(default)]
	pub(crate) expected_evidence_ids: Vec<String>,
	#[serde(default)]
	pub(crate) allowed_alternate_evidence_ids: Vec<String>,
}
impl QueryCase {
	pub(crate) fn generated(
		id: String,
		query: String,
		expected_doc: String,
		expected_terms: Vec<String>,
	) -> Self {
		Self {
			id,
			task: None,
			query,
			expected_evidence_ids: vec![crate::evidence_id_for_doc(&expected_doc)],
			allowed_alternate_docs: Vec::new(),
			allowed_alternate_evidence_ids: Vec::new(),
			expected_doc,
			expected_terms,
		}
	}
}

#[derive(Debug, Serialize)]
pub(crate) struct QuerySummary {
	pub(crate) total: usize,
	pub(crate) pass: usize,
	pub(crate) fail: usize,
	pub(crate) wrong_result_count: usize,
	pub(crate) latency_ms_total: f64,
	pub(crate) latency_ms_mean: f64,
	pub(crate) latency_ms_p50: f64,
	pub(crate) latency_ms_p95: f64,
	pub(crate) latency_ms_p99: f64,
	pub(crate) latency_ms_max: f64,
}

#[derive(Debug, Serialize)]
pub(crate) struct QueryResult {
	pub(crate) id: String,
	pub(crate) task: Option<String>,
	pub(crate) trace_id: Uuid,
	pub(crate) query: String,
	pub(crate) expected_doc: String,
	pub(crate) allowed_alternate_docs: Vec<String>,
	pub(crate) expected_terms: Vec<String>,
	pub(crate) expected_evidence_ids: Vec<String>,
	pub(crate) allowed_alternate_evidence_ids: Vec<String>,
	pub(crate) matched: bool,
	pub(crate) matched_terms: Vec<String>,
	pub(crate) top_evidence_id: Option<String>,
	pub(crate) matched_evidence_id: Option<String>,
	pub(crate) top_note_key: Option<String>,
	pub(crate) top_snippet: Option<String>,
	pub(crate) latency_ms: f64,
	pub(crate) returned_count: usize,
}
