use std::env;

use crate::{
	BTreeSet, ExportQuantitativeAuditManifestArgs, Path, PathBuf, QuantitativeAuditArtifact,
	QuantitativeAuditManifest, RealWorldJob, Result, eyre, fs,
};

use super::{
	QUANTITATIVE_AUDIT_MANIFEST_SCHEMA, REQUIRED_CANDIDATE_LEAKAGE_AUDIT_CONTROL,
	REQUIRED_HELD_OUT_AUDIT_CONTROL, REQUIRED_QREL_LEAKAGE_AUDIT_CONTROL,
	explicit_qrel_query_count, quantitative_corpus_id, ranking_query_count, ranking_query_ids,
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

	let corpus_id = quantitative_corpus_id(jobs);
	let ranking_query_count = ranking_query_count(jobs);
	let explicit_qrel_query_count = explicit_qrel_query_count(jobs);
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
		query_ids: ranking_query_ids(jobs).into_iter().map(str::to_string).collect(),
		controls: args.controls.clone(),
		artifacts: vec![QuantitativeAuditArtifact {
			role: "product_runtime_fixtures".to_string(),
			path: audit_artifact_display_path(args.fixtures.as_path()),
			sha256: fixture_path_digest(args.fixtures.as_path())?,
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

	validate_quantitative_audit_artifacts(manifest, path)
}

fn validate_quantitative_audit_query_ids(
	manifest: &QuantitativeAuditManifest,
	path: &Path,
	source_jobs: &[RealWorldJob],
) -> Result<()> {
	let expected = ranking_query_ids(source_jobs);
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

fn validate_quantitative_audit_artifacts(
	manifest: &QuantitativeAuditManifest,
	path: &Path,
) -> Result<()> {
	if manifest.artifacts.is_empty() {
		return Err(eyre::eyre!("{} has no quantitative audit artifacts.", path.display()));
	}

	for artifact in &manifest.artifacts {
		if artifact.role.trim().is_empty()
			|| artifact.path.trim().is_empty()
			|| artifact.sha256.trim().is_empty()
		{
			return Err(eyre::eyre!(
				"{} has an incomplete quantitative audit artifact.",
				path.display()
			));
		}
		if artifact.sha256.len() != 64 || !artifact.sha256.chars().all(|ch| ch.is_ascii_hexdigit())
		{
			return Err(eyre::eyre!(
				"{} artifact {} has invalid sha256 digest {}.",
				path.display(),
				artifact.role,
				artifact.sha256
			));
		}

		let artifact_path = resolve_quantitative_audit_artifact_path(path, artifact.path.as_str());
		let actual = fixture_path_digest(artifact_path.as_path()).map_err(|err| {
			eyre::eyre!(
				"{} artifact {} could not be digested at {}: {err}",
				path.display(),
				artifact.role,
				artifact_path.display()
			)
		})?;

		if actual != artifact.sha256 {
			return Err(eyre::eyre!(
				"{} artifact {} sha256 mismatch for {}: manifest {}, actual {}.",
				path.display(),
				artifact.role,
				artifact_path.display(),
				artifact.sha256,
				actual
			));
		}
	}

	Ok(())
}

fn resolve_quantitative_audit_artifact_path(manifest_path: &Path, artifact_path: &str) -> PathBuf {
	let raw = PathBuf::from(artifact_path);

	if raw.is_absolute() {
		return raw;
	}

	let cwd_path = env::current_dir().map(|cwd| cwd.join(&raw)).unwrap_or_else(|_| raw.clone());

	if cwd_path.exists() {
		return cwd_path;
	}

	manifest_path.parent().map(|parent| parent.join(&raw)).unwrap_or(cwd_path)
}

fn fixture_path_digest(path: &Path) -> Result<String> {
	let mut hasher = blake3::Hasher::new();

	if path.is_file() {
		hash_fixture_file(
			path,
			path.file_name().and_then(|name| name.to_str()).unwrap_or("fixture"),
			&mut hasher,
		)?;

		return Ok(hasher.finalize().to_hex().to_string());
	}

	let paths = audit_fixture_paths(path)?;

	for fixture in paths {
		let relative = fixture
			.strip_prefix(path)
			.map(|relative| relative.to_string_lossy().replace('\\', "/"))
			.unwrap_or_else(|_| fixture.to_string_lossy().replace('\\', "/"));

		hash_fixture_file(fixture.as_path(), relative.as_str(), &mut hasher)?;
	}

	Ok(hasher.finalize().to_hex().to_string())
}

fn audit_fixture_paths(path: &Path) -> Result<Vec<PathBuf>> {
	let mut paths = Vec::new();

	collect_audit_fixture_paths(path, &mut paths)?;

	paths.sort();

	Ok(paths)
}

fn collect_audit_fixture_paths(path: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
	if path.is_file() {
		paths.push(path.to_path_buf());

		return Ok(());
	}

	for entry in fs::read_dir(path)? {
		let entry_path = entry?.path();

		if entry_path.is_dir() {
			collect_audit_fixture_paths(entry_path.as_path(), paths)?;
		} else if entry_path.extension().and_then(|ext| ext.to_str()) == Some("json") {
			paths.push(entry_path);
		}
	}

	Ok(())
}

fn hash_fixture_file(path: &Path, logical_path: &str, hasher: &mut blake3::Hasher) -> Result<()> {
	hasher.update(logical_path.as_bytes());
	hasher.update(b"\0");
	hasher.update(&fs::read(path)?);
	hasher.update(b"\0");

	Ok(())
}

fn audit_artifact_display_path(path: &Path) -> String {
	let display_path = if path.is_absolute() {
		env::current_dir()
			.ok()
			.and_then(|cwd| path.strip_prefix(cwd).ok().map(Path::to_path_buf))
			.unwrap_or_else(|| path.to_path_buf())
	} else {
		path.to_path_buf()
	};

	display_path.to_string_lossy().replace('\\', "/")
}
