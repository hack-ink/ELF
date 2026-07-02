mod context;
mod schema;

use crate::{
	Path, QuantitativeAuditManifest, Result, quantitative::audit_manifest::QuantitativeAuditContext,
};

pub(super) fn validate_quantitative_audit_identity(
	manifest: &QuantitativeAuditManifest,
	path: &Path,
	context: &QuantitativeAuditContext<'_>,
) -> Result<()> {
	schema::validate_quantitative_audit_schema(manifest, path)?;

	context::validate_quantitative_audit_context(manifest, path, context)
}
