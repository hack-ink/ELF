use crate::{
	BTreeSet, Path, QuantitativeAuditManifest, Result, eyre,
	quantitative::{
		REQUIRED_CANDIDATE_LEAKAGE_AUDIT_CONTROL, REQUIRED_HELD_OUT_AUDIT_CONTROL,
		REQUIRED_QREL_LEAKAGE_AUDIT_CONTROL,
	},
};

pub(super) fn validate_quantitative_audit_controls(
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
