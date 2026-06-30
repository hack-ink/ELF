use crate::validation::{
	self, AuthorityRecoveryDrillArtifact, BTreeSet, Path, REQUIRED_AUTHORITY_PLANES, Result, eyre,
};

pub(in crate::validation::recovery_artifact) fn validate_recovery_authority_record_counts(
	drill: &AuthorityRecoveryDrillArtifact,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	let present_planes = drill
		.authority_record_counts
		.iter()
		.map(|count| count.plane.as_str())
		.collect::<BTreeSet<_>>();

	for plane in REQUIRED_AUTHORITY_PLANES {
		if !present_planes.contains(plane) {
			return Err(eyre::eyre!(
				"{} authority recovery drill {} is missing {} authority counts.",
				path.display(),
				drill.drill_id,
				plane
			));
		}
	}
	for count in &drill.authority_record_counts {
		if count.plane.trim().is_empty() || count.evidence_refs.is_empty() {
			return Err(eyre::eyre!(
				"{} authority recovery drill {} has incomplete authority record counts.",
				path.display(),
				drill.drill_id
			));
		}
		if count.before_count != count.after_count {
			return Err(eyre::eyre!(
				"{} authority recovery drill {} lost or gained {} authority records.",
				path.display(),
				drill.drill_id,
				count.plane
			));
		}
		if !count.source_refs_preserved {
			return Err(eyre::eyre!(
				"{} authority recovery drill {} did not preserve {} authority source refs.",
				path.display(),
				drill.drill_id,
				count.plane
			));
		}
		if !count.lifecycle_history_preserved {
			return Err(eyre::eyre!(
				"{} authority recovery drill {} did not preserve {} authority lifecycle history.",
				path.display(),
				drill.drill_id,
				count.plane
			));
		}

		validation::ensure_known_evidence_refs(path, evidence_ids, &count.evidence_refs)?;
	}

	Ok(())
}
