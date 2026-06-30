use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::provenance::types::rows::NoteIngestDecisionRow;

/// One recorded ingestion decision for a note.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceIngestDecision {
	/// Decision identifier.
	pub decision_id: Uuid,
	/// Tenant that owns the decision record.
	pub tenant_id: String,
	/// Project that owns the decision record.
	pub project_id: String,
	/// Agent that triggered the ingestion decision.
	pub agent_id: String,
	/// Scope key evaluated by the decision.
	pub scope: String,
	/// Pipeline name that produced the decision.
	pub pipeline: String,
	/// Note type discriminator under evaluation.
	pub note_type: String,
	/// Optional application-defined key under evaluation.
	pub note_key: Option<String>,
	/// Note identifier, when a note was persisted or matched.
	pub note_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Note version produced by this decision, when applicable.
	pub note_version_id: Option<Uuid>,
	/// Pre-policy base decision.
	pub base_decision: String,
	/// Final policy decision.
	pub policy_decision: String,
	/// Persistence operation that followed the decision.
	pub note_op: String,
	/// Machine-readable reason code, if any.
	pub reason_code: Option<String>,
	/// Structured diagnostic details.
	pub details: Value,
	#[serde(with = "crate::time_serde")]
	/// Decision timestamp.
	pub ts: OffsetDateTime,
}
impl From<NoteIngestDecisionRow> for NoteProvenanceIngestDecision {
	fn from(row: NoteIngestDecisionRow) -> Self {
		Self {
			decision_id: row.decision_id,
			tenant_id: row.tenant_id,
			project_id: row.project_id,
			agent_id: row.agent_id,
			scope: row.scope,
			pipeline: row.pipeline,
			note_type: row.note_type,
			note_key: row.note_key,
			note_id: row.note_id,
			note_version_id: row.note_version_id,
			base_decision: row.base_decision,
			policy_decision: row.policy_decision,
			note_op: row.note_op,
			reason_code: row.reason_code,
			details: row.details,
			ts: row.ts,
		}
	}
}
