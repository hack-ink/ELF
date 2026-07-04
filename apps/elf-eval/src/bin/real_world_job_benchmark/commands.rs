use crate::{
	AdapterReport, BTreeSet, CaptureIntegrationReport, CorpusProfile,
	ExportQuantitativeAuditManifestArgs, ExportQuantitativeProductManifestArgs, OffsetDateTime,
	Path, PathBuf, PrivateCorpusRedaction, PublishArgs, QuantitativeReportInput, REPORT_SCHEMA,
	RealWorldJob, RealWorldReport, Rfc3339, RunArgs, TypedStatus, VERSION,
	ValidateLocalOrganizerArgs, ValidateSourceBackedQualityArgs, eyre, fs,
};

pub(super) fn run_command(args: RunArgs) -> crate::Result<()> {
	let jobs = load_jobs(&args.fixtures)?;
	let report = build_report(&jobs, &args)?;
	let json = serde_json::to_string_pretty(&report)?;

	write_or_print(args.out.as_deref(), json.as_str())
}

pub(super) fn publish_command(args: PublishArgs) -> crate::Result<()> {
	let raw = fs::read_to_string(&args.report)?;
	let report = serde_json::from_str::<RealWorldReport>(&raw)?;
	let markdown = crate::render_markdown(&report, &args.report);

	write_or_print(args.out.as_deref(), markdown.as_str())
}

pub(super) fn validate_source_backed_quality_command(
	args: ValidateSourceBackedQualityArgs,
) -> crate::Result<()> {
	let raw = fs::read_to_string(&args.report)?;
	let report = serde_json::from_str::<RealWorldReport>(&raw)?;

	crate::validate_source_backed_quality_gate(&report.source_backed_quality).map_err(|failures| {
		eyre::eyre!("source-backed quality gate failed: {}", failures.join(", "))
	})
}

pub(super) fn validate_local_organizer_command(
	args: ValidateLocalOrganizerArgs,
) -> crate::Result<()> {
	let raw = fs::read_to_string(&args.report)?;
	let report = serde_json::from_str::<RealWorldReport>(&raw)?;

	validate_local_organizer_gate(&report)
		.map_err(|failures| eyre::eyre!("local organizer gate failed: {}", failures.join(", ")))
}

pub(super) fn export_quantitative_product_manifest_command(
	args: ExportQuantitativeProductManifestArgs,
) -> crate::Result<()> {
	let raw = fs::read_to_string(&args.report)?;
	let report = serde_json::from_str::<RealWorldReport>(&raw)?;
	let manifest = crate::quantitative_product_manifest_from_report(&report, &args)?;
	let json = serde_json::to_string_pretty(&manifest)?;

	write_or_print(args.out.as_deref(), json.as_str())
}

pub(super) fn export_quantitative_audit_manifest_command(
	args: ExportQuantitativeAuditManifestArgs,
) -> crate::Result<()> {
	let jobs = load_jobs(&args.fixtures)?;
	let manifest = crate::quantitative_audit_manifest_from_jobs(jobs.as_slice(), &args)?;
	let json = serde_json::to_string_pretty(&manifest)?;

	write_or_print(args.out.as_deref(), json.as_str())
}

fn load_jobs(path: &Path) -> crate::Result<Vec<RealWorldJob>> {
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

fn fixture_paths(path: &Path) -> crate::Result<Vec<PathBuf>> {
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

fn collect_fixture_paths(path: &Path, paths: &mut Vec<PathBuf>) -> crate::Result<()> {
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

fn build_report(jobs: &[RealWorldJob], args: &RunArgs) -> crate::Result<RealWorldReport> {
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
	let source_backed_quality =
		crate::source_backed_quality_report(jobs, &job_reports, &summary, &scoreboard);
	let operational_evidence = crate::operational_evidence_report(jobs, &job_reports);
	let adapter = adapter_report(args)?;
	let generated_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
	let quantitative_scoreboard = crate::quantitative_scoreboard_report(QuantitativeReportInput {
		run_id: args.run_id.as_str(),
		generated_at: generated_at.as_str(),
		adapter: &adapter,
		source_jobs: jobs,
		jobs: &job_reports,
		summary: &summary,
		product_manifest_path: args.quantitative_product_manifest.as_deref(),
		audit_manifest_path: args.quantitative_audit_manifest.as_deref(),
	})?;

	Ok(RealWorldReport {
		schema: REPORT_SCHEMA.to_string(),
		run_id: args.run_id.clone(),
		generated_at,
		runner_version: VERSION.to_string(),
		corpus_profile: corpus_profile(jobs),
		adapter,
		scoreboard,
		source_backed_quality,
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

fn validate_local_organizer_gate(report: &RealWorldReport) -> std::result::Result<(), Vec<String>> {
	let Some(summary) = &report.summary.local_organizer else {
		return Err(vec!["missing local_organizer summary".to_string()]);
	};
	let mut failures = Vec::new();

	if summary.job_count == 0 {
		failures.push("local_organizer job_count must be greater than zero".to_string());
	}
	if summary.json_schema_valid_count != summary.job_count {
		failures.push("every local organizer output must be valid JSON/schema".to_string());
	}
	if summary.citation_source_ref_coverage < 1.0 {
		failures.push("citation/source-ref coverage must be 1.0".to_string());
	}
	if summary.unsupported_claim_rate > 0.0 {
		failures.push("unsupported claim rate must be 0.0".to_string());
	}
	if summary.stale_correction_delete_score < 1.0 {
		failures.push("stale/correction/delete score must be 1.0".to_string());
	}
	if summary.source_mutation_count > 0 {
		failures.push("source mutation count must be zero".to_string());
	}
	if summary.silent_memory_authority_mutation_count > 0 {
		failures.push("silent Memory Authority mutation count must be zero".to_string());
	}
	if summary.escalation_rate < 1.0 {
		failures.push("required L2 to L3/L4 escalations must complete".to_string());
	}
	if summary.mean_latency_ms.is_none() || summary.p95_latency_ms.is_none() {
		failures.push("mean and p95 latency must be reported".to_string());
	}
	if summary.tiers.is_empty() {
		failures.push("cost/resource footprint by model tier must be reported".to_string());
	}

	for tier in &summary.tiers {
		if tier.mean_latency_ms.is_none() || tier.p95_latency_ms.is_none() {
			failures.push(format!("tier {} must report mean and p95 latency", tier.model_tier));
		}
		if tier.total_cost.as_ref().is_none_or(|cost| {
			cost.currency.as_deref().is_none_or(str::is_empty)
				&& cost.amount.is_none()
				&& cost.input_tokens.is_none()
				&& cost.output_tokens.is_none()
		}) {
			failures.push(format!("tier {} must report cost footprint", tier.model_tier));
		}
		if tier.resource_footprints.is_empty() {
			failures.push(format!("tier {} must report resource footprint", tier.model_tier));
		}
	}
	for job in &report.jobs {
		let Some(local) = &job.local_organizer else {
			continue;
		};

		if job.status != TypedStatus::Pass {
			failures.push(format!("local organizer job {} did not pass", job.job_id));
		}
		if local.model_tier == "L2" && local.required_escalation_count == 0 {
			failures.push(format!(
				"local organizer job {} must encode at least one required L2 to L3/L4 escalation",
				job.job_id
			));
		}
		if local.completed_escalation_count < local.required_escalation_count {
			failures.push(format!(
				"local organizer job {} must complete all required L2 to L3/L4 escalations",
				job.job_id
			));
		}
		if local.extraction_f1.is_none()
			&& local.extraction_f1_blocker.as_deref().is_none_or(str::is_empty)
		{
			failures.push(format!(
				"local organizer job {} must report extraction_f1 or a not-encoded blocker",
				job.job_id
			));
		}
		if local.runtime_commit.as_deref().is_none_or(str::is_empty)
			|| local.reproducibility_provenance.is_empty()
		{
			failures.push(format!(
				"local organizer job {} must report runtime commit and reproducibility provenance",
				job.job_id
			));
		}
	}

	if failures.is_empty() { Ok(()) } else { Err(failures) }
}

fn corpus_profile(jobs: &[RealWorldJob]) -> String {
	let profiles = jobs.iter().map(|job| job.corpus.profile.as_str()).collect::<BTreeSet<_>>();

	if profiles.len() == 1 {
		profiles.into_iter().next().unwrap_or("unknown").to_string()
	} else {
		"mixed".to_string()
	}
}

fn adapter_report(args: &RunArgs) -> crate::Result<AdapterReport> {
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

fn typed_status_from_arg(raw: &str, flag: &str) -> crate::Result<TypedStatus> {
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

fn write_or_print(path: Option<&Path>, content: &str) -> crate::Result<()> {
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
