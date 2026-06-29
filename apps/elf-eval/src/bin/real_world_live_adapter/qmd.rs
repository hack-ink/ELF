use super::*;

pub(super) fn run_qmd(args: QmdArgs) -> color_eyre::Result<()> {
	let jobs = load_jobs(&args.fixtures)?;
	let result = materialize_qmd_jobs(&args, &jobs);
	let materialized = match result {
		Ok(jobs) => jobs,
		Err(err) => failure_jobs(&args.adapter_id, &jobs, "qmd_cli_runtime", err.to_string()),
	};

	write_materialized_output(MaterializedOutput {
		adapter_id: &args.adapter_id,
		adapter_kind: AdapterKind::QmdCliRuntime,
		fixtures: &args.fixtures,
		out_fixtures: &args.out_fixtures,
		evidence_out: &args.evidence_out,
		jobs: &jobs,
		materialized: &materialized,
		command_evidence: vec![CommandEvidence {
			label: "qmd_cli_runtime".to_string(),
			status: aggregate_status(&materialized),
			command: "cargo run -p elf-eval --bin real_world_live_adapter -- qmd".to_string(),
			artifact: Some(args.evidence_out.display().to_string()),
			reason: "qmd live adapter used collection add, update, embed, and query --json."
				.to_string(),
		}],
		metadata: None,
	})
}

fn materialize_qmd_jobs(
	args: &QmdArgs,
	jobs: &[LoadedJob],
) -> color_eyre::Result<Vec<MaterializedJob>> {
	fs::create_dir_all(&args.work_dir)?;

	let log_path = args.work_dir.join("qmd-live-real-world.log");

	ensure_qmd_checkout(args, &log_path)?;

	let mut out = Vec::with_capacity(jobs.len());

	for loaded in jobs {
		out.push(materialize_qmd_job(args, loaded, &log_path)?);
	}

	Ok(out)
}

fn ensure_qmd_checkout(args: &QmdArgs, log_path: &Path) -> color_eyre::Result<()> {
	if !args.qmd_dir.exists() {
		if let Some(parent) = args.qmd_dir.parent() {
			fs::create_dir_all(parent)?;
		}

		run_logged_command(
			"qmd clone",
			Command::new("git")
				.arg("clone")
				.arg("--depth")
				.arg("1")
				.arg(&args.qmd_repo_url)
				.arg(&args.qmd_dir),
			log_path,
		)?;
	}

	run_logged_shell(
		"qmd install",
		&args.qmd_dir,
		"(npm ci || npm install --no-audit --no-fund) && npm run build --if-present",
		log_path,
	)
}

fn materialize_qmd_job(
	args: &QmdArgs,
	loaded: &LoadedJob,
	log_path: &Path,
) -> color_eyre::Result<MaterializedJob> {
	if let Some(job) = declared_encoding_job(&args.adapter_id, loaded) {
		return Ok(job);
	}
	if let Some(job) = not_encoded_job(&args.adapter_id, loaded) {
		return Ok(job);
	}

	let corpus = corpus_texts(loaded)?;
	let job_slug = slug(&loaded.job.job_id);
	let corpus_dir = args.work_dir.join("corpus").join(&job_slug);
	let home_dir = args.work_dir.join("home").join(&job_slug);
	let collection = format!("elfrw-{job_slug}");

	fs::create_dir_all(&corpus_dir)?;
	fs::create_dir_all(&home_dir)?;

	for existing in read_dir_paths(&corpus_dir)? {
		if existing.is_file() {
			fs::remove_file(existing)?;
		}
	}
	for item in &corpus {
		let path = corpus_dir.join(format!("{}.md", slug(&item.evidence_id)));

		fs::write(path, format!("# {}\n\n{}\n", item.evidence_id, item.text))?;
	}

	run_qmd_command(
		"qmd collection add",
		args,
		&home_dir,
		&[
			"collection",
			"add",
			corpus_dir
				.to_str()
				.ok_or_else(|| eyre::eyre!("qmd corpus path is not valid UTF-8."))?,
			"--name",
			collection.as_str(),
		],
		log_path,
	)?;
	run_qmd_command("qmd update", args, &home_dir, &["update"], log_path)?;
	run_qmd_command(
		"qmd embed",
		args,
		&home_dir,
		&["embed", "-f", "-c", collection.as_str()],
		log_path,
	)?;

	let started_at = Instant::now();
	let query = format!("lex: {}\nvec: {}", loaded.job.prompt.content, loaded.job.prompt.content);
	let stdout = run_qmd_command(
		"qmd query",
		args,
		&home_dir,
		&[
			"query",
			query.as_str(),
			"-c",
			collection.as_str(),
			"--json",
			"--no-rerank",
			"--min-score",
			"0",
			"-n",
			"5",
		],
		log_path,
	)?;
	let latency_ms = started_at.elapsed().as_secs_f64() * 1_000.0;
	let results = serde_json::from_str::<serde_json::Value>(&stdout).map_err(|err| {
		eyre::eyre!("qmd query did not return JSON for {}: {err}", loaded.job.job_id)
	})?;
	let entries = results.as_array().cloned().unwrap_or_default();
	let mut evidence_ids = Vec::new();

	for entry in &entries {
		let entry_text = serde_json::to_string(entry)?;

		for item in &corpus {
			if entry_text.contains(format!("{}.md", slug(&item.evidence_id)).as_str())
				|| entry_text.contains(item.evidence_id.as_str())
			{
				push_unique(&mut evidence_ids, item.evidence_id.clone());
			}
		}
	}

	let selected = selected_required_corpus_texts(loaded, &corpus, &evidence_ids);
	let replay_command = qmd_replay_command(&loaded.job.prompt.content, collection.as_str());
	let (operator_debug, operator_debug_evidence) = operator_debug_output(
		AdapterKind::QmdCliRuntime,
		loaded,
		None,
		replay_command,
		log_path.display().to_string(),
	);

	Ok(qmd_materialized_job(
		loaded,
		&args.adapter_id,
		selected,
		latency_ms,
		entries.len(),
		operator_debug,
		operator_debug_evidence,
	))
}

fn qmd_materialized_job(
	loaded: &LoadedJob,
	adapter_id: &str,
	selected: SelectedEvidenceText,
	latency_ms: f64,
	returned_count: usize,
	operator_debug: Option<serde_json::Value>,
	operator_debug_evidence: Option<OperatorDebugMaterializationEvidence>,
) -> MaterializedJob {
	materialized_job(
		loaded,
		adapter_id,
		MaterializedJobInput {
			content: selected.content,
			evidence_ids: selected.evidence_ids,
			pages: Vec::new(),
			latency_ms,
			indexing_latency_ms: None,
			returned_count,
			trace_id: None,
			failure: None,
			source_mappings: Vec::new(),
			operator_debug,
			operator_debug_evidence,
			capture: None,
			capture_failure: None,
			consolidation_response: None,
			consolidation: None,
			knowledge: None,
			temporal_reconciliation: None,
			dreaming_readback: None,
			memory_summaries: Vec::new(),
			proactive_briefs: Vec::new(),
			scheduled_tasks: Vec::new(),
			trace_stages: None,
		},
	)
}
