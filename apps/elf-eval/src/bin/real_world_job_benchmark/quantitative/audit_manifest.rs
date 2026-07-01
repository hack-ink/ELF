mod artifacts;

use crate::{
	BTreeSet, ExportQuantitativeAuditManifestArgs, Path, QuantitativeAuditArtifact,
	QuantitativeAuditManifest, RealWorldJob, Result, eyre, fs,
	quantitative::{
		QUANTITATIVE_AUDIT_MANIFEST_SCHEMA, REQUIRED_CANDIDATE_LEAKAGE_AUDIT_CONTROL,
		REQUIRED_HELD_OUT_AUDIT_CONTROL, REQUIRED_QREL_LEAKAGE_AUDIT_CONTROL, metrics,
	},
};

pub(super) struct QuantitativeAuditContext<'a> {
	pub(super) run_id: &'a str,
	pub(super) corpus_id: &'a str,
	pub(super) product: &'a str,
	pub(super) adapter_id: &'a str,
	pub(super) source_jobs: &'a [RealWorldJob],
	pub(super) ranking_query_count: usize,
	pub(super) explicit_qrel_query_count: usize,
}

pub(super) struct QuantitativeAuditEvidence {
	pub(super) held_out: bool,
	pub(super) leakage_audited: bool,
	pub(super) audit_manifest_id: Option<String>,
}

pub(crate) fn quantitative_audit_manifest_from_jobs(
	jobs: &[RealWorldJob],
	args: &ExportQuantitativeAuditManifestArgs,
) -> Result<QuantitativeAuditManifest> {
	let product = args.product.trim();
	let adapter_id = args.adapter_id.trim();

	if product.is_empty() || adapter_id.is_empty() {
		return Err(eyre::eyre!("quantitative audit export requires product and adapter_id."));
	}

	let corpus_id = super::quantitative_corpus_id(jobs);
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
		claim_boundary: args.claim_boundary.clone().unwrap_or_else(|| {
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
		}),
	};

	validate_quantitative_audit_manifest(
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

	validate_quantitative_audit_manifest(&manifest, path, context)?;

	Ok(QuantitativeAuditEvidence {
		held_out: manifest.held_out,
		leakage_audited: manifest.leakage_audited,
		audit_manifest_id: Some(manifest.manifest_id),
	})
}

fn validate_quantitative_audit_manifest(
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
