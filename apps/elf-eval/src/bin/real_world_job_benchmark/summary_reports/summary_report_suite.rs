use crate::{Deserialize, Serialize, TypedStatus};

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct SuiteReport {
	pub(crate) suite_id: String,
	pub(crate) status: TypedStatus,
	pub(crate) encoded_job_count: usize,
	pub(crate) score_mean: Option<f64>,
	pub(crate) unsupported_claim_count: usize,
	pub(crate) wrong_result_count: usize,
	#[serde(default)]
	pub(crate) stale_answer_count: usize,
	#[serde(default)]
	pub(crate) conflict_detection_count: usize,
	#[serde(default)]
	pub(crate) update_rationale_available_count: usize,
	#[serde(default)]
	pub(crate) temporal_validity_not_encoded_count: usize,
	#[serde(default)]
	pub(crate) history_readback_encoded_count: usize,
	pub(crate) expected_evidence_recall: Option<f64>,
	pub(crate) irrelevant_context_ratio: Option<f64>,
	pub(crate) trace_explainability_count: usize,
	pub(crate) reason: String,
}
