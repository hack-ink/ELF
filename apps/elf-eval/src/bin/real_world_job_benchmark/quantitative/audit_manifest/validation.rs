use crate::{
	BTreeSet, Path, QuantitativeAuditManifest, RealWorldJob, Result, eyre,
	quantitative::{
		QUANTITATIVE_AUDIT_MANIFEST_SCHEMA, REQUIRED_CANDIDATE_LEAKAGE_AUDIT_CONTROL,
		REQUIRED_HELD_OUT_AUDIT_CONTROL, REQUIRED_QREL_LEAKAGE_AUDIT_CONTROL,
		audit_manifest::{QuantitativeAuditContext, artifacts},
		metrics,
	},
};

pub(super) fn validate_quantitative_audit_manifest(
	manifest: &QuantitativeAuditManifest,
	path: &Path,
	context: QuantitativeAuditContext<'_>,
) -> Result<()> {
	if manifest.schema != QUANTITATIVE_AUDIT_MANIFEST_SCHEMA {
		return Err(eyre::eyre!(
			"{} has schema {}, expected {QUANTITATIVE_AUDIT_MANIFEST_SCHEMA}.",
			path.display(),
			manifest.schema
		));
	}
	if manifest.manifest_id.trim().is_empty() {
		return Err(eyre::eyre!("{} has an empty manifest_id.", path.display()));
	}
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

	validate_quantitative_audit_query_ids(manifest, path, context.source_jobs)?;
	validate_quantitative_audit_controls(manifest, path)?;

	artifacts::validate_quantitative_audit_artifacts(manifest, path)
}

fn validate_quantitative_audit_query_ids(
	manifest: &QuantitativeAuditManifest,
	path: &Path,
	source_jobs: &[RealWorldJob],
) -> Result<()> {
	let expected = metrics::ranking_query_ids(source_jobs);
	let actual = manifest.query_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();

	if actual.len() != manifest.query_ids.len() {
		return Err(eyre::eyre!("{} has duplicate quantitative audit query_ids.", path.display()));
	}
	if actual != expected {
		let missing = expected.difference(&actual).copied().collect::<Vec<_>>();
		let extra = actual.difference(&expected).copied().collect::<Vec<_>>();

		return Err(eyre::eyre!(
			"{} audit query_ids do not match current ranked-query set; missing: {:?}, extra: {:?}.",
			path.display(),
			missing,
			extra
		));
	}

	Ok(())
}

fn validate_quantitative_audit_controls(
	manifest: &QuantitativeAuditManifest,
	path: &Path,
) -> Result<()> {
	let controls = manifest.controls.iter().map(String::as_str).collect::<BTreeSet<_>>();

	if manifest.held_out && !controls.contains(REQUIRED_HELD_OUT_AUDIT_CONTROL) {
		return Err(eyre::eyre!(
			"{} marks held_out=true without required control {}.",
			path.display(),
			REQUIRED_HELD_OUT_AUDIT_CONTROL
		));
	}
	if manifest.leakage_audited
		&& (!controls.contains(REQUIRED_QREL_LEAKAGE_AUDIT_CONTROL)
			|| !controls.contains(REQUIRED_CANDIDATE_LEAKAGE_AUDIT_CONTROL))
	{
		return Err(eyre::eyre!(
			"{} marks leakage_audited=true without required controls {} and {}.",
			path.display(),
			REQUIRED_QREL_LEAKAGE_AUDIT_CONTROL,
			REQUIRED_CANDIDATE_LEAKAGE_AUDIT_CONTROL
		));
	}
	if (manifest.held_out || manifest.leakage_audited) && manifest.claim_boundary.trim().is_empty()
	{
		return Err(eyre::eyre!(
			"{} marks audit controls true but has an empty claim_boundary.",
			path.display()
		));
	}

	Ok(())
}
