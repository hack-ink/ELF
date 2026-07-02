mod controls;
mod identity;
mod queries;

use crate::{
	Path, QuantitativeAuditManifest, Result,
	quantitative::audit_manifest::{QuantitativeAuditContext, artifacts},
};

pub(super) fn validate_quantitative_audit_manifest(
	manifest: &QuantitativeAuditManifest,
	path: &Path,
	context: QuantitativeAuditContext<'_>,
) -> Result<()> {
	identity::validate_quantitative_audit_identity(manifest, path, &context)?;
	queries::validate_quantitative_audit_query_ids(manifest, path, context.source_jobs)?;
	controls::validate_quantitative_audit_controls(manifest, path)?;

	artifacts::validate_quantitative_audit_artifacts(manifest, path)
}
