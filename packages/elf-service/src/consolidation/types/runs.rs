use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::consolidation::types::{
	ConsolidationProposalInput, ConsolidationProposalResponse, empty_object,
};
use elf_domain::consolidation::{ConsolidationInputRef, ConsolidationLineage};
use elf_storage::models::ConsolidationRun;

/// Request to create a fixture-backed consolidation run.
#[derive(Clone, Debug, Deserialize)]
pub struct ConsolidationRunCreateRequest {
	/// Tenant that owns the run.
	pub tenant_id: String,
	/// Project that owns the run.
	pub project_id: String,
	/// Agent registering the run.
	pub agent_id: String,
	/// Job kind, such as `fixture` or `manual`.
	pub job_kind: String,
	/// Input references considered by the run.
	pub input_refs: Vec<ConsolidationInputRef>,
	#[serde(default = "empty_object")]
	/// Aggregate source snapshot metadata for the run.
	pub source_snapshot: Value,
	/// Run lineage.
	pub lineage: ConsolidationLineage,
	#[serde(default)]
	/// Fixture-generated proposals to persist with this run.
	pub proposals: Vec<ConsolidationProposalInput>,
}

/// Response returned after creating one consolidation run.
#[derive(Clone, Debug, Serialize)]
pub struct ConsolidationRunCreateResponse {
	/// Created run.
	pub run: ConsolidationRunResponse,
	/// Enqueued worker job identifier.
	pub job_id: Uuid,
	/// Proposals stored with the run.
	pub proposals: Vec<ConsolidationProposalResponse>,
}

/// Request to get one consolidation run.
#[derive(Clone, Debug, Deserialize)]
pub struct ConsolidationRunGetRequest {
	/// Tenant that owns the run.
	pub tenant_id: String,
	/// Project that owns the run.
	pub project_id: String,
	/// Run identifier.
	pub run_id: Uuid,
}

/// Request to list consolidation runs.
#[derive(Clone, Debug, Deserialize)]
pub struct ConsolidationRunsListRequest {
	/// Tenant that owns the runs.
	pub tenant_id: String,
	/// Project that owns the runs.
	pub project_id: String,
	/// Maximum number of runs to return.
	pub limit: Option<u32>,
}

/// Response returned by consolidation run listing.
#[derive(Clone, Debug, Serialize)]
pub struct ConsolidationRunsListResponse {
	/// Returned runs.
	pub runs: Vec<ConsolidationRunResponse>,
}

/// Public consolidation run DTO.
#[derive(Clone, Debug, Serialize)]
pub struct ConsolidationRunResponse {
	/// Consolidation run identifier.
	pub run_id: Uuid,
	/// Tenant that owns the run.
	pub tenant_id: String,
	/// Project that owns the run.
	pub project_id: String,
	/// Agent that registered the run.
	pub agent_id: String,
	/// Versioned consolidation contract schema.
	pub contract_schema: String,
	/// Job kind, such as fixture or manual.
	pub job_kind: String,
	/// Current run state.
	pub status: String,
	/// Serialized input references.
	pub input_refs: Value,
	/// Aggregate source snapshot metadata.
	pub source_snapshot: Value,
	/// Serialized run lineage.
	pub lineage: Value,
	/// Structured error payload for failed runs.
	pub error: Value,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
	/// Completion timestamp for terminal runs.
	pub completed_at: Option<OffsetDateTime>,
}
impl From<ConsolidationRun> for ConsolidationRunResponse {
	fn from(run: ConsolidationRun) -> Self {
		Self {
			run_id: run.run_id,
			tenant_id: run.tenant_id,
			project_id: run.project_id,
			agent_id: run.agent_id,
			contract_schema: run.contract_schema,
			job_kind: run.job_kind,
			status: run.status,
			input_refs: run.input_refs,
			source_snapshot: run.source_snapshot,
			lineage: run.lineage,
			error: run.error,
			created_at: run.created_at,
			updated_at: run.updated_at,
			completed_at: run.completed_at,
		}
	}
}
