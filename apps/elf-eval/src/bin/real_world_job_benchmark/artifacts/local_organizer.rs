use crate::{CostReport, Deserialize, Serialize, Value};

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct LocalOrganizerFixture {
	pub(crate) model_tier: String,
	pub(crate) model_profile_id: String,
	pub(crate) runtime_id: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) runtime_commit: Option<String>,
	#[serde(default)]
	pub(crate) reproducibility_provenance: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) extraction_f1: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) extraction_f1_blocker: Option<String>,
	pub(crate) json_schema_valid: bool,
	pub(crate) citation_source_ref_coverage: f64,
	pub(crate) unsupported_claim_rate: f64,
	pub(crate) stale_correction_delete_score: f64,
	#[serde(default)]
	pub(crate) source_mutation_count: usize,
	#[serde(default)]
	pub(crate) silent_memory_authority_mutation_count: usize,
	#[serde(default)]
	pub(crate) required_escalation_count: usize,
	#[serde(default)]
	pub(crate) completed_escalation_count: usize,
	#[serde(default)]
	pub(crate) resource_footprint: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct LocalOrganizerJobReport {
	pub(crate) model_tier: String,
	pub(crate) model_profile_id: String,
	pub(crate) runtime_id: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) runtime_commit: Option<String>,
	pub(crate) reproducibility_provenance: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) extraction_f1: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) extraction_f1_blocker: Option<String>,
	pub(crate) json_schema_valid: bool,
	pub(crate) citation_source_ref_coverage: f64,
	pub(crate) unsupported_claim_rate: f64,
	pub(crate) stale_correction_delete_score: f64,
	pub(crate) source_mutation_count: usize,
	pub(crate) silent_memory_authority_mutation_count: usize,
	pub(crate) required_escalation_count: usize,
	pub(crate) completed_escalation_count: usize,
	pub(crate) escalation_rate: f64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) latency_ms: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) cost: Option<CostReport>,
	pub(crate) resource_footprint: Value,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct LocalOrganizerSummaryReport {
	pub(crate) job_count: usize,
	pub(crate) extraction_f1_not_encoded_count: usize,
	pub(crate) json_schema_valid_count: usize,
	pub(crate) citation_source_ref_coverage: f64,
	pub(crate) unsupported_claim_rate: f64,
	pub(crate) stale_correction_delete_score: f64,
	pub(crate) source_mutation_count: usize,
	pub(crate) silent_memory_authority_mutation_count: usize,
	pub(crate) escalation_rate: f64,
	pub(crate) mean_latency_ms: Option<f64>,
	pub(crate) p95_latency_ms: Option<f64>,
	pub(crate) tiers: Vec<LocalOrganizerTierReport>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct LocalOrganizerTierReport {
	pub(crate) model_tier: String,
	pub(crate) job_count: usize,
	pub(crate) mean_latency_ms: Option<f64>,
	pub(crate) p95_latency_ms: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) total_cost: Option<CostReport>,
	pub(crate) resource_footprints: Vec<Value>,
}
