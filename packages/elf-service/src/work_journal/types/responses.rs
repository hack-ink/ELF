use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::work_journal::types::WorkJournalEntryFamily;
use elf_domain::writegate::WritePolicyAudit;

/// Response payload after Work Journal capture.
#[derive(Clone, Debug, Serialize)]
pub struct WorkJournalEntryCreateResponse {
	/// Stored Work Journal entry.
	pub entry: WorkJournalEntryResponse,
}

/// Session-level Work Journal readback.
#[derive(Clone, Debug, Serialize)]
pub struct WorkJournalSessionReadbackResponse {
	/// Readback schema identifier.
	pub schema: String,
	/// Stable session identifier.
	pub session_id: String,
	/// Newest-first journal entries.
	pub items: Vec<WorkJournalEntryResponse>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Compact "where did we stop" projection from the returned entries.
	pub where_stopped: Option<WorkJournalWhereStopped>,
}

/// One source-adjacent Work Journal entry returned by readback.
#[derive(Clone, Debug, Serialize)]
pub struct WorkJournalEntryResponse {
	/// Readback schema identifier.
	pub schema: String,
	/// Journal entry identifier.
	pub entry_id: Uuid,
	/// Tenant that owns the entry.
	pub tenant_id: String,
	/// Project that owns the entry.
	pub project_id: String,
	/// Agent that captured the entry.
	pub agent_id: String,
	/// Visibility scope for readback.
	pub scope: String,
	/// Stable session identifier.
	pub session_id: String,
	/// Entry family.
	pub family: WorkJournalEntryFamily,
	/// Lifecycle status.
	pub status: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional display title.
	pub title: Option<String>,
	/// Redacted durable journal body.
	pub body: String,
	/// Source refs supporting the entry.
	pub source_refs: Vec<Value>,
	/// Explicit next steps stated by the captured source.
	pub explicit_next_steps: Vec<String>,
	/// Inferred next steps retained as non-authoritative hints.
	pub inferred_next_steps: Vec<String>,
	/// Rejected options captured by the journal.
	pub rejected_options: Vec<String>,
	/// Promotion boundary metadata.
	pub promotion_boundary: Value,
	/// Redaction audit for the durable journal body.
	pub redaction_audit: WritePolicyAudit,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Compact "where did we stop" projection for one journal session.
#[derive(Clone, Debug, Serialize)]
pub struct WorkJournalWhereStopped {
	/// Latest returned entry identifier.
	pub latest_entry_id: Uuid,
	/// Latest returned entry family.
	pub latest_family: WorkJournalEntryFamily,
	/// Source refs associated with the latest returned entry.
	pub source_refs: Vec<Value>,
	/// Most recent explicit next steps in returned entries.
	pub explicit_next_steps: Vec<String>,
	/// Most recent inferred next steps in returned entries.
	pub inferred_next_steps: Vec<String>,
	/// Most recent rejected options in returned entries.
	pub rejected_options: Vec<String>,
	/// Promotion boundary for the latest returned entry.
	pub promotion_boundary: Value,
}
