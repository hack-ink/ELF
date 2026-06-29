use super::*;

pub(super) fn validate_scheduled_memory_artifact(
	task: &ScheduledMemoryTaskArtifact,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if task.task_run_id.trim().is_empty()
		|| task.contract_schema != "elf.scheduled_memory_task/v1"
		|| task.generated_at.trim().is_empty()
		|| task.scheduled_for.trim().is_empty()
		|| task.tenant_id.trim().is_empty()
		|| task.project_id.trim().is_empty()
		|| task.agent_id.trim().is_empty()
		|| task.read_profile.trim().is_empty()
		|| task.task_kind.trim().is_empty()
		|| task.outputs.is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete scheduled memory task.", path.display()));
	}
	if !is_scheduled_task_kind(task.task_kind.as_str()) {
		return Err(eyre::eyre!(
			"{} has unknown scheduled task kind {}.",
			path.display(),
			task.task_kind
		));
	}

	validate_optional_rfc3339(&task.generated_at, path, task.task_run_id.as_str())?;
	validate_optional_rfc3339(&task.scheduled_for, path, task.task_run_id.as_str())?;

	for output in &task.outputs {
		validate_scheduled_memory_output(output, path, evidence_ids)?;
	}
	for mutation in &task.source_mutations {
		if !mutation.is_object() {
			return Err(eyre::eyre!(
				"{} scheduled memory source mutations must be JSON objects.",
				path.display()
			));
		}
	}
	for flag in &task.unsupported_claim_flags {
		if !flag.is_object() {
			return Err(eyre::eyre!(
				"{} scheduled memory unsupported-claim flags must be JSON objects.",
				path.display()
			));
		}
	}

	validate_memory_summary_source_trace(&task.source_trace, path, evidence_ids)?;

	if let Some(trace) = &task.execution_trace {
		validate_scheduled_memory_trace(trace, path, evidence_ids)?;
	}

	Ok(())
}

fn validate_scheduled_memory_output(
	output: &ScheduledMemoryOutput,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if output.output_id.trim().is_empty()
		|| output.output_kind.trim().is_empty()
		|| output.text.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete scheduled memory output.", path.display()));
	}
	if !is_scheduled_task_kind(output.output_kind.as_str()) {
		return Err(eyre::eyre!(
			"{} has unknown scheduled output kind {}.",
			path.display(),
			output.output_kind
		));
	}
	if !is_memory_summary_freshness_status(output.freshness.status.as_str()) {
		return Err(eyre::eyre!(
			"{} has unknown scheduled output freshness status {}.",
			path.display(),
			output.freshness.status
		));
	}
	if !is_proactive_action_decision(output.action.decision.as_str()) {
		return Err(eyre::eyre!(
			"{} has unknown scheduled output action decision {}.",
			path.display(),
			output.action.decision
		));
	}
	if output.action.reason_code.trim().is_empty() || output.action.reason.trim().is_empty() {
		return Err(eyre::eyre!(
			"{} has incomplete scheduled output action rationale.",
			path.display()
		));
	}

	for evidence_id in &output.evidence_refs {
		ensure_known_evidence(path, evidence_ids, evidence_id)?;
	}
	for evidence_id in &output.freshness.tombstone_refs {
		ensure_known_evidence(path, evidence_ids, evidence_id)?;
	}
	for flag in &output.unsupported_claim_flags {
		if !flag.is_object() {
			return Err(eyre::eyre!(
				"{} scheduled output unsupported-claim flags must be JSON objects.",
				path.display()
			));
		}
	}

	validate_optional_summary_time(
		path,
		output.freshness.observed_at.as_deref(),
		output.output_id.as_str(),
	)?;
	validate_optional_summary_time(
		path,
		output.freshness.valid_from.as_deref(),
		output.output_id.as_str(),
	)?;
	validate_optional_summary_time(
		path,
		output.freshness.valid_to.as_deref(),
		output.output_id.as_str(),
	)?;
	validate_optional_summary_time(
		path,
		output.freshness.last_confirmed_at.as_deref(),
		output.output_id.as_str(),
	)?;

	Ok(())
}

fn validate_scheduled_memory_trace(
	trace: &ScheduledMemoryExecutionTrace,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if trace.trace_id.trim().is_empty()
		|| trace.trigger_kind.trim().is_empty()
		|| trace.status.trim().is_empty()
		|| trace.started_at.trim().is_empty()
		|| trace.completed_at.trim().is_empty()
		|| trace.output_ref.trim().is_empty()
	{
		return Err(eyre::eyre!(
			"{} has an incomplete scheduled memory execution trace.",
			path.display()
		));
	}

	validate_optional_rfc3339(&trace.started_at, path, trace.trace_id.as_str())?;
	validate_optional_rfc3339(&trace.completed_at, path, trace.trace_id.as_str())?;

	for stage in &trace.stages {
		if stage.stage_name.trim().is_empty() || stage.summary.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} has an incomplete scheduled memory trace stage.",
				path.display()
			));
		}

		for evidence_id in &stage.evidence_refs {
			ensure_known_evidence(path, evidence_ids, evidence_id)?;
		}
	}

	Ok(())
}
