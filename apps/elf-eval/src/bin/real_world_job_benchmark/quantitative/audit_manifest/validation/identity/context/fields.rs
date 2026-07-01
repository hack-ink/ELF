use crate::{
	Path, QuantitativeAuditManifest, Result, eyre,
	quantitative::audit_manifest::QuantitativeAuditContext,
};

pub(super) fn validate_quantitative_audit_context_fields(
	manifest: &QuantitativeAuditManifest,
	path: &Path,
	context: &QuantitativeAuditContext<'_>,
) -> Result<()> {
	if manifest.run_id != context.run_id {
		return Err(eyre::eyre!(
			"{} has run_id {}, expected {}.",
			path.display(),
			manifest.run_id,
			context.run_id
		));
	}
	if manifest.corpus_id != context.corpus_id {
		return Err(eyre::eyre!(
			"{} has corpus_id {}, expected {}.",
			path.display(),
			manifest.corpus_id,
			context.corpus_id
		));
	}
	if manifest.product != context.product || manifest.adapter_id != context.adapter_id {
		return Err(eyre::eyre!(
			"{} has product {}:{} but current row is {}:{}.",
			path.display(),
			manifest.product,
			manifest.adapter_id,
			context.product,
			context.adapter_id
		));
	}

	Ok(())
}
