use crate::validation::{self, BTreeSet, Path, RecoveryMeasurement, Result, eyre};

pub(in crate::validation::recovery_artifact) fn validate_recovery_measurement(
	label: &str,
	measurement: &RecoveryMeasurement,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if !measurement.target_seconds.is_finite()
		|| !measurement.measured_seconds.is_finite()
		|| measurement.target_seconds < 0.0
		|| measurement.measured_seconds < 0.0
		|| measurement.evidence_refs.is_empty()
	{
		return Err(eyre::eyre!("{} has invalid {label} recovery measurement.", path.display()));
	}
	if !validation::recovery_measurement_met(measurement) {
		return Err(eyre::eyre!("{} exceeded {label} recovery target.", path.display()));
	}

	validation::ensure_known_evidence_refs(path, evidence_ids, &measurement.evidence_refs)
}
