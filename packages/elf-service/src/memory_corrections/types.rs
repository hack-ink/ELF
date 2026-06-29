use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::NoteOp;

/// Review-backed correction action for an approved memory record.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCorrectionAction {
	/// Mark the memory as superseded while retaining historical readback.
	Supersede,
	/// Tombstone the memory while retaining historical readback.
	Delete,
	/// Restore the latest prior active snapshot from the memory ledger.
	Restore,
}
impl MemoryCorrectionAction {
	/// Returns the canonical action string.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Supersede => "supersede",
			Self::Delete => "delete",
			Self::Restore => "restore",
		}
	}
}

/// Request payload for applying a memory correction.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryCorrectionRequest {
	/// Tenant that owns the memory.
	pub tenant_id: String,
	/// Project that owns the memory.
	pub project_id: String,
	/// Reviewer or policy actor applying the correction.
	pub actor_agent_id: String,
	/// Identifier of the memory note being corrected.
	pub note_id: Uuid,
	/// Correction action to apply.
	pub action: MemoryCorrectionAction,
	/// Reviewer or policy reason for the correction.
	pub reason: String,
	/// Source reference or review record that justifies the correction.
	pub source_ref: Value,
	/// Optional ledger version to restore from. Defaults to the latest supersede/delete snapshot.
	pub restore_version_id: Option<Uuid>,
}

/// Response returned after applying a memory correction.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryCorrectionResponse {
	/// Identifier of the corrected memory note.
	pub note_id: Uuid,
	/// Correction action that was requested.
	pub action: MemoryCorrectionAction,
	/// Storage operation applied to the memory record.
	pub op: NoteOp,
	/// Current lifecycle status after the correction.
	pub status: String,
	/// Version row written for this correction, when a change occurred.
	pub version_id: Option<Uuid>,
}
