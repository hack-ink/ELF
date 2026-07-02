use crate::{
	BTreeSet, Path, QuantitativeAuditManifest, RealWorldJob, Result, eyre, quantitative::metrics,
};

pub(super) fn validate_quantitative_audit_query_ids(
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
