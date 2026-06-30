use crate::{
	AdapterKind, Instant, LoadedJob, MaterializedJob, Path, QmdArgs, Result, eyre, fs,
	qmd::response,
};

pub(super) fn materialize_qmd_job(
	args: &QmdArgs,
	loaded: &LoadedJob,
	log_path: &Path,
) -> Result<MaterializedJob> {
	if let Some(job) = crate::declared_encoding_job(&args.adapter_id, loaded) {
		return Ok(job);
	}
	if let Some(job) = crate::not_encoded_job(&args.adapter_id, loaded) {
		return Ok(job);
	}

	let corpus = crate::corpus_texts(loaded)?;
	let job_slug = crate::slug(&loaded.job.job_id);
	let corpus_dir = args.work_dir.join("corpus").join(&job_slug);
	let home_dir = args.work_dir.join("home").join(&job_slug);
	let collection = format!("elfrw-{job_slug}");

	fs::create_dir_all(&corpus_dir)?;
	fs::create_dir_all(&home_dir)?;

	for existing in crate::read_dir_paths(&corpus_dir)? {
		if existing.is_file() {
			fs::remove_file(existing)?;
		}
	}
	for item in &corpus {
		let path = corpus_dir.join(format!("{}.md", crate::slug(&item.evidence_id)));

		fs::write(path, format!("# {}\n\n{}\n", item.evidence_id, item.text))?;
	}

	crate::run_qmd_command(
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
	crate::run_qmd_command("qmd update", args, &home_dir, &["update"], log_path)?;
	crate::run_qmd_command(
		"qmd embed",
		args,
		&home_dir,
		&["embed", "-f", "-c", collection.as_str()],
		log_path,
	)?;

	let started_at = Instant::now();
	let query = format!("lex: {}\nvec: {}", loaded.job.prompt.content, loaded.job.prompt.content);
	let stdout = crate::run_qmd_command(
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
	let (entries, evidence_ids) = response::qmd_query_entries(loaded, &corpus, &stdout)?;
	let selected = crate::selected_required_corpus_texts(loaded, &corpus, &evidence_ids);
	let replay_command = crate::qmd_replay_command(&loaded.job.prompt.content, collection.as_str());
	let (operator_debug, operator_debug_evidence) = crate::operator_debug_output(
		AdapterKind::QmdCliRuntime,
		loaded,
		None,
		replay_command,
		log_path.display().to_string(),
	);

	Ok(response::qmd_materialized_job(
		loaded,
		&args.adapter_id,
		selected,
		latency_ms,
		entries.len(),
		operator_debug,
		operator_debug_evidence,
	))
}
