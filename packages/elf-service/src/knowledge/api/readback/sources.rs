use crate::knowledge::api::{KnowledgePageSourceRef, OffsetDateTime, Serialize, Uuid, Value};

/// Readback DTO for one normalized source reference.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageSourceRefResponse {
	/// Source-reference row identifier.
	pub ref_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Citing section, when section-scoped.
	pub section_id: Option<Uuid>,
	/// Source kind.
	pub source_kind: String,
	/// Authoritative source identifier.
	pub source_id: Uuid,
	/// Captured source status.
	pub source_status: Option<String>,
	/// Captured source update timestamp.
	pub source_updated_at: Option<OffsetDateTime>,
	/// Captured source content hash.
	pub source_content_hash: Option<String>,
	/// Captured source snapshot.
	pub source_snapshot: Value,
	/// Citation-local metadata.
	pub citation_metadata: Value,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}
impl From<KnowledgePageSourceRef> for KnowledgePageSourceRefResponse {
	fn from(source_ref: KnowledgePageSourceRef) -> Self {
		Self {
			ref_id: source_ref.ref_id,
			page_id: source_ref.page_id,
			section_id: source_ref.section_id,
			source_kind: source_ref.source_kind,
			source_id: source_ref.source_id,
			source_status: source_ref.source_status,
			source_updated_at: source_ref.source_updated_at,
			source_content_hash: source_ref.source_content_hash,
			source_snapshot: source_ref.source_snapshot,
			citation_metadata: source_ref.citation_metadata,
			created_at: source_ref.created_at,
		}
	}
}
