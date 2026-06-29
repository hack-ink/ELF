use crate::validation::{self, Path, RealWorldJob, Result, TypedStatus, eyre};

pub(super) fn validate_scoring_rubric(job: &RealWorldJob, path: &Path) -> Result<()> {
	if !(0.0..=1.0).contains(&job.scoring_rubric.pass_threshold) {
		return Err(eyre::eyre!("{} has invalid pass_threshold.", path.display()));
	}
	if job.scoring_rubric.dimensions.is_empty() {
		return Err(eyre::eyre!("{} has no scoring dimensions.", path.display()));
	}

	for (dimension_id, dimension) in &job.scoring_rubric.dimensions {
		if dimension_id.trim().is_empty()
			|| !dimension.weight.is_finite()
			|| !dimension.max_points.is_finite()
			|| dimension.weight <= 0.0
			|| dimension.max_points <= 0.0
			|| dimension.criteria.is_null()
		{
			return Err(eyre::eyre!(
				"{} has invalid scoring dimension {}.",
				path.display(),
				dimension_id
			));
		}
	}
	for rule in &job.scoring_rubric.hard_fail_rules {
		if rule.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty hard fail rule.", path.display()));
		}
	}

	Ok(())
}

pub(super) fn validate_allowed_uncertainty(job: &RealWorldJob, path: &Path) -> Result<()> {
	if job.allowed_uncertainty.fallback_action.trim().is_empty() {
		return Err(eyre::eyre!("{} has an empty fallback action.", path.display()));
	}
	if job.allowed_uncertainty.can_answer_unknown
		&& job.allowed_uncertainty.acceptable_phrases.is_empty()
	{
		return Err(eyre::eyre!(
			"{} allows unknown answers but defines no acceptable uncertainty phrase.",
			path.display()
		));
	}

	for phrase in &job.allowed_uncertainty.acceptable_phrases {
		if phrase.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty uncertainty phrase.", path.display()));
		}
	}

	Ok(())
}

pub(super) fn validate_operator_debug(job: &RealWorldJob, path: &Path) -> Result<()> {
	let Some(debug) = &job.operator_debug else {
		if job.suite == "operator_debugging_ux" {
			return Err(eyre::eyre!(
				"{} operator_debugging_ux job must include operator_debug.",
				path.display()
			));
		}

		return Ok(());
	};

	if debug.failure_mode.trim().is_empty()
		|| debug.root_cause.trim().is_empty()
		|| debug.dropped_candidate_visibility.trim().is_empty()
		|| debug.trace_completeness.trim().is_empty()
		|| debug.repair_action_clarity.trim().is_empty()
		|| debug.steps_to_root_cause == 0
	{
		return Err(eyre::eyre!("{} has incomplete operator_debug evidence.", path.display()));
	}

	validate_optional_debug_field(path, debug.trace_id.as_deref(), "trace_id")?;
	validate_optional_debug_field(path, debug.viewer_url.as_deref(), "viewer_url")?;
	validate_optional_debug_field(
		path,
		debug.admin_trace_bundle_url.as_deref(),
		"admin_trace_bundle_url",
	)?;
	validate_optional_debug_field(path, debug.replay_command.as_deref(), "replay_command")?;
	validate_optional_debug_field(path, debug.replay_artifact.as_deref(), "replay_artifact")?;
	validate_non_empty_debug_list(path, &debug.viewer_panels, "viewer_panels")?;
	validate_non_empty_debug_list(path, &debug.cli_steps, "cli_steps")?;
	validate_non_empty_debug_list(path, &debug.trace_evidence, "trace_evidence")?;

	for gap in &debug.ux_gaps {
		if gap.gap_id.trim().is_empty()
			|| gap.severity.trim().is_empty()
			|| gap.description.trim().is_empty()
			|| gap.follow_up_issue.trim().is_empty()
		{
			return Err(eyre::eyre!("{} has incomplete operator_debug ux_gaps.", path.display()));
		}
	}

	Ok(())
}

pub(super) fn validate_job_encoding(job: &RealWorldJob, path: &Path) -> Result<()> {
	if let Some(status) = job.encoding.status {
		if !matches!(
			status,
			TypedStatus::NotEncoded | TypedStatus::Blocked | TypedStatus::Incomplete
		) {
			return Err(eyre::eyre!(
				"{} job {} uses encoding.status {}; only not_encoded, blocked, or incomplete are allowed.",
				path.display(),
				job.job_id,
				validation::status_str(status)
			));
		}
		if job.encoding.reason.as_deref().is_none_or(|reason| reason.trim().is_empty()) {
			return Err(eyre::eyre!(
				"{} job {} declares encoding.status but no reason.",
				path.display(),
				job.job_id
			));
		}
	}
	if let Some(follow_up) = &job.encoding.follow_up
		&& (follow_up.title.trim().is_empty() || follow_up.reason.trim().is_empty())
	{
		return Err(eyre::eyre!(
			"{} job {} has an incomplete encoding follow-up.",
			path.display(),
			job.job_id
		));
	}

	Ok(())
}

fn validate_optional_debug_field(path: &Path, value: Option<&str>, field: &str) -> Result<()> {
	if value.is_some_and(|value| value.trim().is_empty()) {
		return Err(eyre::eyre!("{} has empty operator_debug {field}.", path.display()));
	}

	Ok(())
}

fn validate_non_empty_debug_list(path: &Path, values: &[String], field: &str) -> Result<()> {
	if values.iter().any(|value| value.trim().is_empty()) {
		return Err(eyre::eyre!("{} has empty operator_debug {field} entry.", path.display()));
	}

	Ok(())
}
