use crate::{
	Path, QuantitativeAuditManifest, Result, eyre,
	quantitative::audit_manifest::QuantitativeAuditContext,
};

pub(super) fn validate_quantitative_audit_context(
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
	if manifest.sample_size != context.source_jobs.len() {
		return Err(eyre::eyre!(
			"{} has sample_size {}, expected {}.",
			path.display(),
			manifest.sample_size,
			context.source_jobs.len()
		));
	}
	if manifest.ranking_query_count != context.ranking_query_count {
		return Err(eyre::eyre!(
			"{} has ranking_query_count {}, expected {}.",
			path.display(),
			manifest.ranking_query_count,
			context.ranking_query_count
		));
	}
	if manifest.explicit_qrel_query_count != context.explicit_qrel_query_count {
		return Err(eyre::eyre!(
			"{} has explicit_qrel_query_count {}, expected {}.",
			path.display(),
			manifest.explicit_qrel_query_count,
			context.explicit_qrel_query_count
		));
	}

	Ok(())
}
