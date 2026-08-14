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
			let file_name = opaque_source_name(&item.evidence_id);
			let artifact_path = corpus_dir.join(&file_name);
			let file_source = format!("elf-real-world/{run_slug}/{job_slug}/{file_name}");

			fs::write(&artifact_path, format!("{}\n", item.text.trim_end()))?;

			Ok(LightragSource { evidence_id: item.evidence_id.clone(), file_source })
		})
		.collect()
}

pub(super) fn lightrag_keywords(query: &str) -> Vec<String> {
	crate::terms(query).into_iter().take(12).collect()
}

fn opaque_source_name(evidence_id: &str) -> String {
	let digest = blake3::hash(evidence_id.as_bytes()).to_hex();

	format!("source-{}.md", &digest.as_str()[..24])
}

#[cfg(test)]
mod tests {
	#[test]
	fn source_name_is_stable_and_does_not_expose_evidence_id() {
		let evidence_id = "customer-decision-2026-07";
		let name = super::opaque_source_name(evidence_id);

		assert_eq!(name, super::opaque_source_name(evidence_id));
		assert!(name.starts_with("source-"));
		assert!(!name.contains(evidence_id));
	}
}
