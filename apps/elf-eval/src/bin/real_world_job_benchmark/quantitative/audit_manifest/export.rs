use crate::{
	ExportQuantitativeAuditManifestArgs, QuantitativeAuditArtifact, QuantitativeAuditManifest,
	RealWorldJob, Result, eyre,
	quantitative::{
		self, QUANTITATIVE_AUDIT_MANIFEST_SCHEMA,
		audit_manifest::{QuantitativeAuditContext, artifacts, validation},
		metrics,
	},
};

pub(crate) fn quantitative_audit_manifest_from_jobs(
	jobs: &[RealWorldJob],
	args: &ExportQuantitativeAuditManifestArgs,
) -> Result<QuantitativeAuditManifest> {
	let product = args.product.trim();
	let adapter_id = args.adapter_id.trim();

	if product.is_empty() || adapter_id.is_empty() {
		return Err(eyre::eyre!("quantitative audit export requires product and adapter_id."));
	}

	let corpus_id = quantitative::quantitative_corpus_id(jobs);
	let ranking_query_count = metrics::ranking_query_count(jobs);
	let explicit_qrel_query_count = metrics::explicit_qrel_query_count(jobs);
	let manifest = QuantitativeAuditManifest {
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
		claim_boundary: quantitative_audit_claim_boundary(args),
	};

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

fn quantitative_audit_claim_boundary(args: &ExportQuantitativeAuditManifestArgs) -> String {
	args.claim_boundary.clone().unwrap_or_else(|| {
		if args.held_out || args.leakage_audited {
			concat!(
				"Audit manifest supplied by operator; runner validates run/corpus/product/",
				"adapter/count/query-id/artifact bindings before opening row gates."
			)
			.to_string()
		} else {
			concat!(
				"Diagnostic audit manifest binds the current product-runtime fixture set to ",
				"query ids and counts, but it does not prove held-out or leakage-audited status."
			)
			.to_string()
		}
	})
}
