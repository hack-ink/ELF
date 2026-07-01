use crate::{
	ExportQuantitativeAuditManifestArgs, QuantitativeAuditArtifact, QuantitativeAuditManifest,
	RealWorldJob, Result,
	quantitative::{
		self, QUANTITATIVE_AUDIT_MANIFEST_SCHEMA,
		audit_manifest::{artifacts, export::claim_boundary},
		metrics,
	},
};

pub(super) fn quantitative_audit_manifest(
	jobs: &[RealWorldJob],
	args: &ExportQuantitativeAuditManifestArgs,
	product: &str,
	adapter_id: &str,
) -> Result<QuantitativeAuditManifest> {
	let corpus_id = quantitative::quantitative_corpus_id(jobs);
	let ranking_query_count = metrics::ranking_query_count(jobs);
	let explicit_qrel_query_count = metrics::explicit_qrel_query_count(jobs);

	Ok(QuantitativeAuditManifest {
		schema: QUANTITATIVE_AUDIT_MANIFEST_SCHEMA.to_string(),
		manifest_id: args
			.manifest_id
			.clone()
			.unwrap_or_else(|| format!("{}-quantitative-audit-manifest", args.run_id)),
		run_id: args.run_id.clone(),
		corpus_id,
		product: product.to_string(),
		adapter_id: adapter_id.to_string(),
		held_out: args.held_out,
		leakage_audited: args.leakage_audited,
		sample_size: jobs.len(),
		ranking_query_count,
		explicit_qrel_query_count,
		query_ids: metrics::ranking_query_ids(jobs).into_iter().map(str::to_string).collect(),
		controls: args.controls.clone(),
		artifacts: vec![QuantitativeAuditArtifact {
			role: "product_runtime_fixtures".to_string(),
			path: artifacts::audit_artifact_display_path(args.fixtures.as_path()),
			sha256: artifacts::fixture_path_digest(args.fixtures.as_path())?,
		}],
		claim_boundary: claim_boundary::quantitative_audit_claim_boundary(args),
	})
}
