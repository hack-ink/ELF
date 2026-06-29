use super::*;

pub(super) fn load_jobs(path: &Path) -> color_eyre::Result<Vec<LoadedJob>> {
	let paths = fixture_paths(path)?;
	let mut jobs = Vec::with_capacity(paths.len());

	for fixture in paths {
		let raw = fs::read_to_string(&fixture)?;
		let value = serde_json::from_str::<serde_json::Value>(&raw)
			.map_err(|err| eyre::eyre!("Failed to parse {} as JSON: {err}", fixture.display()))?;
		let job = serde_json::from_value::<LiveJob>(value.clone()).map_err(|err| {
			eyre::eyre!("Failed to parse {} as real_world_job: {err}", fixture.display())
		})?;

		if job.schema != JOB_SCHEMA {
			return Err(eyre::eyre!(
				"{} has schema {}, expected {JOB_SCHEMA}.",
				fixture.display(),
				job.schema
			));
		}
		if job.corpus.items.is_empty() {
			return Err(eyre::eyre!("{} has no corpus items.", fixture.display()));
		}

		jobs.push(LoadedJob { path: fixture, value, job });
	}

	Ok(jobs)
}

fn fixture_paths(path: &Path) -> color_eyre::Result<Vec<PathBuf>> {
	let mut paths = Vec::new();

	collect_fixture_paths(path, &mut paths)?;

	paths.sort();

	Ok(paths)
}

fn collect_fixture_paths(path: &Path, paths: &mut Vec<PathBuf>) -> color_eyre::Result<()> {
	if path.is_dir() {
		for entry in fs::read_dir(path)? {
			let entry_path = entry?.path();

			collect_fixture_paths(entry_path.as_path(), paths)?;
		}

		return Ok(());
	}
	if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
		paths.push(path.to_path_buf());
	}

	Ok(())
}

pub(super) fn corpus_texts(loaded: &LoadedJob) -> color_eyre::Result<Vec<CorpusText>> {
	loaded
		.job
		.corpus
		.items
		.iter()
		.map(|item| {
			let text = match (&item.text, &item.local_ref) {
				(Some(text), _) => text.clone(),
				(None, Some(local_ref)) => {
					let base = loaded.path.parent().unwrap_or_else(|| Path::new("."));

					fs::read_to_string(base.join(local_ref))?
				},
				(None, None) => {
					return Err(eyre::eyre!(
						"{} item {} has no text or local_ref.",
						loaded.path.display(),
						item.evidence_id
					));
				},
			};

			Ok(CorpusText {
				evidence_id: item.evidence_id.clone(),
				text: text.trim().to_string(),
				capture: item.capture.clone(),
			})
		})
		.collect()
}

pub(super) fn read_dir_paths(path: &Path) -> color_eyre::Result<Vec<PathBuf>> {
	if !path.exists() {
		return Ok(Vec::new());
	}

	let mut paths = Vec::new();

	for entry in fs::read_dir(path)? {
		paths.push(entry?.path());
	}

	Ok(paths)
}
