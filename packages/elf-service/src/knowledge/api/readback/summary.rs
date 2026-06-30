use crate::knowledge::api::{self, KnowledgePage, OffsetDateTime, Serialize, Uuid, Value};

/// Summary DTO for one derived knowledge page.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageSummary {
	/// Page identifier.
	pub page_id: Uuid,
	/// Tenant that owns the page.
	pub tenant_id: String,
	/// Project that owns the page.
	pub project_id: String,
	/// Page kind.
	pub page_kind: String,
	/// Stable page key.
	pub page_key: String,
	/// Page title.
	pub title: String,
	/// Versioned page contract schema.
	pub contract_schema: String,
	/// Page lifecycle status.
	pub status: String,
	/// Canonical source snapshot hash.
	pub rebuild_source_hash: String,
	/// Canonical page content hash.
	pub content_hash: String,
	/// Source coverage metadata.
	pub source_coverage: Value,
	/// Rebuild metadata.
	pub rebuild_metadata: Value,
	/// Previous-version diff metadata, when present.
	pub previous_version_diff: Option<Value>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
	/// Last rebuild timestamp.
	pub rebuilt_at: OffsetDateTime,
}
impl From<KnowledgePage> for KnowledgePageSummary {
	fn from(page: KnowledgePage) -> Self {
		Self {
			page_id: page.page_id,
			tenant_id: page.tenant_id,
			project_id: page.project_id,
			page_kind: page.page_kind,
			page_key: page.page_key,
			title: page.title,
			contract_schema: page.contract_schema,
			status: page.status,
			rebuild_source_hash: page.rebuild_source_hash,
			content_hash: page.content_hash,
			source_coverage: page.source_coverage,
			previous_version_diff: api::previous_version_diff_from_metadata(&page.rebuild_metadata),
			rebuild_metadata: page.rebuild_metadata,
			created_at: page.created_at,
			updated_at: page.updated_at,
			rebuilt_at: page.rebuilt_at,
		}
	}
}
