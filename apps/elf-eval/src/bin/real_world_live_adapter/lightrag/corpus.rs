use std::fs;

use crate::{CorpusText, LightragArgs, LightragSource, LoadedJob, Result};

pub(super) fn write_lightrag_corpus(
	args: &LightragArgs,
	loaded: &LoadedJob,
	corpus: &[CorpusText],
	run_slug: &str,
) -> Result<Vec<LightragSource>> {
	let job_slug = crate::slug(&loaded.job.job_id);
	let corpus_dir = args.work_dir.join("corpus").join(run_slug).join(&job_slug);

	fs::create_dir_all(&corpus_dir)?;

	corpus
		.iter()
		.map(|item| {
			let file_name = format!("{}.md", crate::slug(&item.evidence_id));
			let artifact_path = corpus_dir.join(&file_name);
			let file_source = format!("elf-real-world/{run_slug}/{job_slug}/{file_name}");

			fs::write(&artifact_path, format!("# {}\n\n{}\n", item.evidence_id, item.text))?;

			Ok(LightragSource { evidence_id: item.evidence_id.clone(), file_source, artifact_path })
		})
		.collect()
}

pub(super) fn lightrag_keywords(query: &str) -> Vec<String> {
	crate::terms(query).into_iter().take(12).collect()
}
