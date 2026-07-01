use crate::{
	AdapterReport, BTreeSet, CaptureIntegrationReport, CorpusProfile,
	ExportQuantitativeProductManifestArgs, OffsetDateTime, Path, PathBuf, PrivateCorpusRedaction,
	PublishArgs, QuantitativeReportInput, REPORT_SCHEMA, RealWorldJob, RealWorldReport, Result,
	Rfc3339, RunArgs, TypedStatus, VERSION, eyre, fs,
};

pub(super) fn run_command(args: RunArgs) -> Result<()> {
	let jobs = load_jobs(&args.fixtures)?;
	let report = build_report(&jobs, &args)?;
	let json = serde_json::to_string_pretty(&report)?;

	write_or_print(args.out.as_deref(), json.as_str())
}

pub(super) fn publish_command(args: PublishArgs) -> Result<()> {
	let raw = fs::read_to_string(&args.report)?;
	let report = serde_json::from_str::<RealWorldReport>(&raw)?;
	let markdown = crate::render_markdown(&report, &args.report);

	write_or_print(args.out.as_deref(), markdown.as_str())
}

pub(super) fn export_quantitative_product_manifest_command(
	args: ExportQuantitativeProductManifestArgs,
) -> Result<()> {
	let raw = fs::read_to_string(&args.report)?;
	let report = serde_json::from_str::<RealWorldReport>(&raw)?;
	let manifest = crate::quantitative_product_manifest_from_report(&report, &args)?;
	let json = serde_json::to_string_pretty(&manifest)?;

	write_or_print(args.out.as_deref(), json.as_str())
}

fn load_jobs(path: &Path) -> Result<Vec<RealWorldJob>> {
	let paths = fixture_paths(path)?;
	let mut jobs = Vec::with_capacity(paths.len());

	for fixture in paths {
		let raw = fs::read_to_string(&fixture)?;
		let job = serde_json::from_str::<RealWorldJob>(&raw)
			.map_err(|err| eyre::eyre!("Failed to parse {}: {err}", fixture.display()))?;

		crate::validate_job(&job, &fixture)?;

		jobs.push(job);
	}

	Ok(jobs)
}

fn fixture_paths(path: &Path) -> Result<Vec<PathBuf>> {
	if path.is_file() {
		return Ok(vec![path.to_path_buf()]);
	}
	if !path.is_dir() {
		return Err(eyre::eyre!("Fixture path does not exist: {}", path.display()));
	}

	let mut paths = Vec::new();

	collect_fixture_paths(path, &mut paths)?;

	paths.sort();

	if paths.is_empty() {
		return Err(eyre::eyre!("No JSON fixtures found in {}.", path.display()));
	}

	Ok(paths)
}

fn collect_fixture_paths(path: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
	for entry in fs::read_dir(path)? {
		let entry = entry?;
		let entry_path = entry.path();

		if entry_path.is_dir() {
			collect_fixture_paths(entry_path.as_path(), paths)?;
		} else if entry_path.extension().and_then(|ext| ext.to_str()) == Some("json") {
			paths.push(entry_path);
		}
	}

	Ok(())
}

fn build_report(jobs: &[RealWorldJob], args: &RunArgs) -> Result<RealWorldReport> {
	if jobs.is_empty() {
		return Err(eyre::eyre!("At least one real_world_job fixture is required."));
	}

	let mut job_reports = Vec::with_capacity(jobs.len());
	let mut unsupported_claims = Vec::new();

	for job in jobs {
		let scoring = crate::score_job(job);

		unsupported_claims.extend(scoring.unsupported_claims.clone());
		job_reports.push(crate::job_report(job, scoring));
	}

	let suites = crate::suite_reports(&job_reports);
	let not_encoded_suites = suites
		.iter()
		.filter(|suite| suite.status == TypedStatus::NotEncoded)
		.map(|suite| suite.suite_id.clone())
		.collect::<Vec<_>>();
	let summary = crate::report_summary(&job_reports, &suites);
	let evolution = crate::evolution_summary(&job_reports);
	let follow_ups = crate::follow_up_reports(jobs);
	let external_adapters = crate::external_adapter_section(
		&args.external_adapter_manifest,
		args.skip_external_adapter_manifest,
	)?;
	let scoreboard = crate::scoreboard_report(jobs, &job_reports, &summary, &external_adapters);
	let operational_evidence = crate::operational_evidence_report(jobs, &job_reports);
	let adapter = adapter_report(args)?;
	let generated_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
	let quantitative_scoreboard = crate::quantitative_scoreboard_report(QuantitativeReportInput {
		generated_at: generated_at.as_str(),
		adapter: &adapter,
		source_jobs: jobs,
		jobs: &job_reports,
		summary: &summary,
		product_manifest_path: args.quantitative_product_manifest.as_deref(),
	})?;

	Ok(RealWorldReport {
		schema: REPORT_SCHEMA.to_string(),
		run_id: args.run_id.clone(),
		generated_at,
		runner_version: VERSION.to_string(),
		corpus_profile: corpus_profile(jobs),
		adapter,
		scoreboard,
		operational_evidence,
		quantitative_scoreboard,
		external_adapters,
		capture_integration: capture_integration_report(jobs),
		summary,
		suites,
		jobs: job_reports,
		unsupported_claims,
		not_encoded_suites,
		private_corpus_redaction: private_corpus_redaction(jobs),
		evolution,
		follow_ups,
	})
}

fn corpus_profile(jobs: &[RealWorldJob]) -> String {
	let profiles = jobs.iter().map(|job| job.corpus.profile.as_str()).collect::<BTreeSet<_>>();

	if profiles.len() == 1 {
		profiles.into_iter().next().unwrap_or("unknown").to_string()
	} else {
		"mixed".to_string()
	}
}

fn adapter_report(args: &RunArgs) -> Result<AdapterReport> {
	Ok(AdapterReport {
		adapter_id: args.adapter_id.clone(),
		name: args.adapter_name.clone(),
		behavior: args.adapter_behavior.clone(),
		storage: typed_status_from_arg(
			args.adapter_storage_status.as_str(),
			"--adapter-storage-status",
		)?,
		runtime: typed_status_from_arg(
			args.adapter_runtime_status.as_str(),
			"--adapter-runtime-status",
		)?,
		notes: args.adapter_notes.clone(),
	})
}

fn typed_status_from_arg(raw: &str, flag: &str) -> Result<TypedStatus> {
	match raw {
		"pass" => Ok(TypedStatus::Pass),
		"wrong_result" => Ok(TypedStatus::WrongResult),
		"lifecycle_fail" => Ok(TypedStatus::LifecycleFail),
		"incomplete" => Ok(TypedStatus::Incomplete),
		"blocked" => Ok(TypedStatus::Blocked),
		"not_encoded" => Ok(TypedStatus::NotEncoded),
		"unsupported_claim" => Ok(TypedStatus::UnsupportedClaim),
		_ => Err(eyre::eyre!(
			"{flag} must be one of pass, wrong_result, lifecycle_fail, incomplete, blocked, not_encoded, or unsupported_claim."
		)),
	}
}

fn capture_integration_report(jobs: &[RealWorldJob]) -> CaptureIntegrationReport {
	let mut report = CaptureIntegrationReport::default();

	for job in jobs {
		extend_unique(&mut report.real, &job.corpus.capture_behaviors.real);
		extend_unique(&mut report.fixture_backed, &job.corpus.capture_behaviors.fixture_backed);
		extend_unique(&mut report.mocked, &job.corpus.capture_behaviors.mocked);
		extend_unique(&mut report.blocked, &job.corpus.capture_behaviors.blocked);
		extend_unique(&mut report.not_encoded, &job.corpus.capture_behaviors.not_encoded);
		extend_unique(&mut report.notes, &job.corpus.capture_behaviors.notes);
	}

	if report.real.is_empty()
		&& report.fixture_backed.is_empty()
		&& report.mocked.is_empty()
		&& report.blocked.is_empty()
		&& report.not_encoded.is_empty()
	{
		report
			.not_encoded
			.push("No capture/integration behavior was declared by encoded fixtures.".to_string());
	}

	report
}

fn extend_unique(target: &mut Vec<String>, values: &[String]) {
	let mut seen = target.iter().cloned().collect::<BTreeSet<_>>();

	for value in values {
		if seen.insert(value.clone()) {
			target.push(value.clone());
		}
	}
}

fn private_corpus_redaction(jobs: &[RealWorldJob]) -> PrivateCorpusRedaction {
	let private_fixture_count = jobs
		.iter()
		.filter(|job| matches!(job.corpus.profile, CorpusProfile::PrivateSanitized))
		.count();
	let policy = if private_fixture_count == 0 {
		"no_private_corpus".to_string()
	} else {
		"publish evidence ids and bounded score summaries only; do not publish private text"
			.to_string()
	};

	PrivateCorpusRedaction { policy, private_fixture_count }
}

fn write_or_print(path: Option<&Path>, content: &str) -> Result<()> {
	if let Some(path) = path {
		if let Some(parent) = path.parent()
			&& !parent.as_os_str().is_empty()
		{
			fs::create_dir_all(parent)?;
		}

		fs::write(path, content)?;

		println!("Wrote {}", path.display());
	} else {
		println!("{content}");
	}

	Ok(())
}
