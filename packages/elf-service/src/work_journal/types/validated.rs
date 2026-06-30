use serde_json::Value;
use uuid::Uuid;

use elf_domain::writegate::WritePolicyAudit;

pub(in crate::work_journal) struct ValidatedWorkJournalCreate {
	pub(in crate::work_journal) entry_id: Uuid,
	pub(in crate::work_journal) scope: String,
	pub(in crate::work_journal) session_id: String,
	pub(in crate::work_journal) title: Option<String>,
	pub(in crate::work_journal) body: String,
	pub(in crate::work_journal) source_refs: Value,
	pub(in crate::work_journal) explicit_next_steps: Value,
	pub(in crate::work_journal) inferred_next_steps: Value,
	pub(in crate::work_journal) rejected_options: Value,
	pub(in crate::work_journal) promotion_boundary: Value,
	pub(in crate::work_journal) redaction_audit: WritePolicyAudit,
}
