use super::*;

pub(super) fn validate_trace_explainability(job: &RealWorldJob, path: &Path) -> Result<()> {
	let Some(trace) = job
		.corpus
		.adapter_response
		.as_ref()
		.and_then(|response| response.answer.trace_explainability.as_ref())
	else {
		return Ok(());
	};
	let known = corpus_evidence_ids(job);
	let stage_names =
		trace.stages.iter().map(|stage| stage.stage_name.as_str()).collect::<BTreeSet<_>>();

	if trace.trace_id.as_deref().is_some_and(str::is_empty) {
		return Err(eyre::eyre!("{} has an empty trace_explainability trace_id.", path.display()));
	}
	if trace.failure_stage.as_deref().is_some_and(str::is_empty) {
		return Err(eyre::eyre!(
			"{} has an empty trace_explainability failure_stage.",
			path.display()
		));
	}

	if let Some(failure_stage) = trace.failure_stage.as_deref()
		&& !stage_names.is_empty()
		&& !stage_names.contains(failure_stage)
	{
		return Err(eyre::eyre!(
			"{} trace_explainability failure_stage {} is not present in stages.",
			path.display(),
			failure_stage
		));
	}

	for stage in &trace.stages {
		validate_trace_stage(stage, &known, path)?;
	}

	Ok(())
}

fn validate_trace_stage(
	stage: &TraceStageExplainability,
	known: &BTreeSet<String>,
	path: &Path,
) -> Result<()> {
	if stage.stage_name.trim().is_empty() {
		return Err(eyre::eyre!("{} has a trace stage with an empty stage_name.", path.display()));
	}

	for evidence_id in stage
		.kept_evidence
		.iter()
		.chain(stage.dropped_evidence.iter())
		.chain(stage.demoted_evidence.iter())
		.chain(stage.distractor_evidence.iter())
	{
		ensure_known_evidence(path, known, evidence_id)?;
	}

	Ok(())
}
