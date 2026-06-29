use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, Result};
use elf_domain::writegate::{WritePolicy, WritePolicyAudit};

/// Schema identifier for Work Journal readback.
pub const ELF_WORK_JOURNAL_SCHEMA_V1: &str = "elf.work_journal/v1";

pub(super) const WORK_JOURNAL_PROMOTION_BOUNDARY_SCHEMA_V1: &str =
	"elf.work_journal.promotion_boundary/v1";
pub(super) const DEFAULT_SESSION_READBACK_LIMIT: u32 = 20;
pub(super) const MAX_SESSION_READBACK_LIMIT: u32 = 100;
pub(super) const MAX_STORAGE_SCAN_ROWS: i64 = 500;
pub(super) const MAX_BODY_CHARS: usize = 16_384;
pub(super) const MAX_SIDE_LIST_ITEMS: usize = 64;

/// Work Journal entry family.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkJournalEntryFamily {
	/// Session log captured alongside source work.
	SessionLog,
	/// Handoff brief for another agent or future session.
	HandoffBrief,
	/// Janitor or cleanup report.
	JanitorReport,
	/// Explicit next step stated in the source.
	ExplicitNextStep,
	/// Inferred next step retained as a non-authoritative hint.
	InferredNextStep,
	/// Option that was considered and rejected.
	RejectedOption,
}
impl WorkJournalEntryFamily {
	/// Returns the canonical API/storage string.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::SessionLog => "session_log",
			Self::HandoffBrief => "handoff_brief",
			Self::JanitorReport => "janitor_report",
			Self::ExplicitNextStep => "explicit_next_step",
			Self::InferredNextStep => "inferred_next_step",
			Self::RejectedOption => "rejected_option",
		}
	}

	pub(super) fn parse(raw: &str) -> Result<Self> {
		match raw {
			"session_log" => Ok(Self::SessionLog),
			"handoff_brief" => Ok(Self::HandoffBrief),
			"janitor_report" => Ok(Self::JanitorReport),
			"explicit_next_step" => Ok(Self::ExplicitNextStep),
			"inferred_next_step" => Ok(Self::InferredNextStep),
			"rejected_option" => Ok(Self::RejectedOption),
			_ => Err(Error::InvalidRequest {
				message: "family must be one of: session_log, handoff_brief, janitor_report, explicit_next_step, inferred_next_step, rejected_option.".to_string(),
			}),
		}
	}
}

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

/// Response payload after Work Journal capture.
#[derive(Clone, Debug, Serialize)]
pub struct WorkJournalEntryCreateResponse {
	/// Stored Work Journal entry.
	pub entry: WorkJournalEntryResponse,
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

pub(super) struct ValidatedWorkJournalCreate {
	pub(super) entry_id: Uuid,
	pub(super) scope: String,
	pub(super) session_id: String,
	pub(super) title: Option<String>,
	pub(super) body: String,
	pub(super) source_refs: Value,
	pub(super) explicit_next_steps: Value,
	pub(super) inferred_next_steps: Value,
	pub(super) rejected_options: Value,
	pub(super) promotion_boundary: Value,
	pub(super) redaction_audit: WritePolicyAudit,
}

fn empty_object() -> Value {
	Value::Object(Map::new())
}
