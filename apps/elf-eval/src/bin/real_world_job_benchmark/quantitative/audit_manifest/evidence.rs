use crate::{
	Path, QuantitativeAuditManifest, Result, eyre, fs,
	quantitative::audit_manifest::{
		QuantitativeAuditContext, QuantitativeAuditEvidence, validation,
	},
};

pub(super) fn quantitative_audit_evidence(
	path: Option<&Path>,
	context: QuantitativeAuditContext<'_>,
) -> Result<QuantitativeAuditEvidence> {
	let Some(path) = path else {
		return Ok(QuantitativeAuditEvidence {
			held_out: false,
			leakage_audited: false,
			audit_manifest_id: None,
		});
	};
	let raw = fs::read_to_string(path)?;
	let manifest = serde_json::from_str::<QuantitativeAuditManifest>(&raw).map_err(|err| {
		eyre::eyre!("Failed to parse quantitative audit manifest {}: {err}", path.display())
	})?;

	validation::validate_quantitative_audit_manifest(&manifest, path, context)?;

	Ok(QuantitativeAuditEvidence {
		held_out: manifest.held_out,
		leakage_audited: manifest.leakage_audited,
		audit_manifest_id: Some(manifest.manifest_id),
	})
}
