use crate::docs::DocType;
use elf_domain::writegate::WritePolicyAudit;

#[derive(Debug)]
pub(in crate::docs) struct ValidatedDocsPut {
	pub(in crate::docs) doc_type: DocType,
	pub(in crate::docs) content: String,
	pub(in crate::docs) write_policy_audit: Option<WritePolicyAudit>,
}
