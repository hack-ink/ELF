use serde_json::{Map, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::docs::DocType;
use elf_domain::writegate::WritePolicyAudit;
use elf_storage::models::DocChunk;

pub(in crate::docs) struct SourceCaptureSummaryInput<'a> {
	pub(in crate::docs) doc_id: Uuid,
	pub(in crate::docs) source_ref: &'a Map<String, Value>,
	pub(in crate::docs) doc_type: DocType,
	pub(in crate::docs) scope: &'a str,
	pub(in crate::docs) title: Option<&'a str>,
	pub(in crate::docs) content_hash: &'a str,
	pub(in crate::docs) raw_content_hash: &'a str,
	pub(in crate::docs) now: OffsetDateTime,
	pub(in crate::docs) chunks: &'a [DocChunk],
	pub(in crate::docs) write_policy_audit: Option<&'a WritePolicyAudit>,
}
