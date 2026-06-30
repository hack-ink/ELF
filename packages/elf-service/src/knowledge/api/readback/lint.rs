use crate::knowledge::api::{
	self, KnowledgePageLintFinding, OffsetDateTime, Serialize, Uuid, Value,
};

/// Readback DTO for one knowledge page lint finding.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageLintFindingResponse {
	/// Lint finding identifier.
	pub finding_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Associated section, when available.
	pub section_id: Option<Uuid>,
	/// Finding type.
	pub finding_type: String,
	/// Finding severity.
	pub severity: String,
	/// Source kind associated with the finding, when available.
	pub source_kind: Option<String>,
	/// Source identifier associated with the finding, when available.
	pub source_id: Option<Uuid>,
	/// Human-readable finding message.
	pub message: String,
	/// Structured finding details.
	pub details: Value,
	/// Operator guidance for repair or rebuild.
	pub repair_guidance: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}
impl From<KnowledgePageLintFinding> for KnowledgePageLintFindingResponse {
	fn from(finding: KnowledgePageLintFinding) -> Self {
		let repair_guidance =
			api::repair_guidance_for_finding_type(finding.finding_type.as_str()).to_string();

		Self {
			finding_id: finding.finding_id,
			page_id: finding.page_id,
			section_id: finding.section_id,
			finding_type: finding.finding_type,
			severity: finding.severity,
			source_kind: finding.source_kind,
			source_id: finding.source_id,
			message: finding.message,
			repair_guidance,
			details: finding.details,
			created_at: finding.created_at,
		}
	}
}
