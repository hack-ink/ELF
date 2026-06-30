use serde::Deserialize;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::work_journal::types::WorkJournalEntryFamily;
use elf_domain::writegate::WritePolicy;

/// Request payload for source-adjacent Work Journal capture.
#[derive(Clone, Debug, Deserialize)]
pub struct WorkJournalEntryCreateRequest {
	/// Tenant that owns the entry.
	pub tenant_id: String,
	/// Project that owns the entry.
	pub project_id: String,
	/// Agent capturing the entry.
	pub agent_id: String,
	/// Optional caller-supplied stable identifier.
	pub entry_id: Option<Uuid>,
	/// Visibility scope for readback.
	pub scope: String,
	/// Stable session identifier for grouping entries.
	pub session_id: String,
	/// Entry family.
	pub family: WorkJournalEntryFamily,
	/// Optional display title.
	pub title: Option<String>,
	/// Journal body. This is source-adjacent, not authoritative memory.
	pub body: String,
	/// Source refs that support the journal entry.
	pub source_refs: Vec<Value>,
	/// Redaction/exclusion policy applied before persistence.
	pub write_policy: Option<WritePolicy>,
	#[serde(default)]
	/// Explicit next steps stated by the captured source.
	pub explicit_next_steps: Vec<String>,
	#[serde(default)]
	/// Inferred next steps retained as non-authoritative hints.
	pub inferred_next_steps: Vec<String>,
	#[serde(default)]
	/// Options considered and rejected during the captured work.
	pub rejected_options: Vec<String>,
	#[serde(default = "empty_object")]
	/// Promotion boundary metadata.
	pub promotion_boundary: Value,
}

/// Request payload for one Work Journal entry lookup.
#[derive(Clone, Debug, Deserialize)]
pub struct WorkJournalEntryGetRequest {
	/// Tenant that owns the entry.
	pub tenant_id: String,
	/// Project used for read-profile and shared-grant checks.
	pub project_id: String,
	/// Agent requesting the read.
	pub agent_id: String,
	/// Read profile that determines visible scopes.
	pub read_profile: String,
	/// Entry identifier.
	pub entry_id: Uuid,
}

/// Request payload for session-level Work Journal readback.
#[derive(Clone, Debug, Deserialize)]
pub struct WorkJournalSessionReadbackRequest {
	/// Tenant that owns the session.
	pub tenant_id: String,
	/// Project used for read-profile and shared-grant checks.
	pub project_id: String,
	/// Agent requesting the read.
	pub agent_id: String,
	/// Read profile that determines visible scopes.
	pub read_profile: String,
	/// Stable session identifier to read.
	pub session_id: String,
	#[serde(default)]
	/// Optional family filter.
	pub families: Vec<WorkJournalEntryFamily>,
	/// Maximum number of returned entries.
	pub limit: Option<u32>,
}

fn empty_object() -> Value {
	Value::Object(Map::new())
}
