mod claim_boundary;
mod identity;
mod manifest;

use crate::{
	ExportQuantitativeAuditManifestArgs, QuantitativeAuditManifest, RealWorldJob, Result,
	quantitative::audit_manifest::{QuantitativeAuditContext, validation},
};

pub(crate) fn quantitative_audit_manifest_from_jobs(
	jobs: &[RealWorldJob],
	args: &ExportQuantitativeAuditManifestArgs,
) -> Result<QuantitativeAuditManifest> {
	let product = args.product.trim();
	let adapter_id = args.adapter_id.trim();

	identity::validate_audit_export_identity(product, adapter_id)?;

	let manifest = manifest::quantitative_audit_manifest(jobs, args, product, adapter_id)?;

	validation::validate_quantitative_audit_manifest(
		&manifest,
		args.fixtures.as_path(),
		QuantitativeAuditContext {
			run_id: args.run_id.as_str(),
			corpus_id: manifest.corpus_id.as_str(),
			product,
			adapter_id,
			source_jobs: jobs,
			ranking_query_count: manifest.ranking_query_count,
			explicit_qrel_query_count: manifest.explicit_qrel_query_count,
		},
	)?;

	Ok(manifest)
}
