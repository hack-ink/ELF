mod counts;
mod fields;

use crate::{
	Path, QuantitativeAuditManifest, Result, quantitative::audit_manifest::QuantitativeAuditContext,
};

pub(super) fn validate_quantitative_audit_context(
	manifest: &QuantitativeAuditManifest,
	path: &Path,
	context: &QuantitativeAuditContext<'_>,
) -> Result<()> {
	fields::validate_quantitative_audit_context_fields(manifest, path, context)?;
	counts::validate_quantitative_audit_context_counts(manifest, path, context)?;

	Ok(())
}
