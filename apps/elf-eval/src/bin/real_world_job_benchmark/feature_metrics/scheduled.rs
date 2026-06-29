use crate::feature_metrics::{
	self, BTreeSet, ProducedAnswer, RealWorldJob, ScheduledMemoryExecutionTrace,
	ScheduledMemoryJobMetrics, ScheduledMemoryOutput, ScheduledMemoryTaskArtifact,
	UnsupportedClaimReport, forbidden_diff_key_count,
};

pub(super) fn scheduled_memory_metrics_impl(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
) -> Option<ScheduledMemoryJobMetrics> {
	if answer.scheduled_tasks.is_empty() {
		return None;
	}

	let mut metrics = ScheduledMemoryJobMetrics {
		task_run_count: answer.scheduled_tasks.len(),
		required_task_kind_count: job
			.scheduled_memory
			.as_ref()
			.map_or(0, |scheduled| scheduled.required_task_kinds.len()),
		..ScheduledMemoryJobMetrics::default()
	};
	let mut task_kinds = BTreeSet::new();

	for task in &answer.scheduled_tasks {
		accumulate_scheduled_memory_metrics(task, &mut metrics, &mut task_kinds);
	}

	let covered_required_task_kind_count = job.scheduled_memory.as_ref().map_or(0, |scheduled| {
		scheduled.required_task_kinds.iter().filter(|kind| task_kinds.contains(*kind)).count()
	});

	metrics.covered_required_task_kind_count = covered_required_task_kind_count;
	metrics.missing_required_task_kind_count =
		metrics.required_task_kind_count.saturating_sub(covered_required_task_kind_count);
	metrics.evidence_ref_coverage = feature_metrics::ratio(
		metrics.evidence_ref_output_count,
		metrics.evidence_ref_required_count,
	);
	metrics.freshness_coverage =
		feature_metrics::ratio(metrics.freshness_marker_count, metrics.output_count);
	metrics.action_rationale_coverage =
		feature_metrics::ratio(metrics.action_rationale_count, metrics.output_count);
	metrics.trace_coverage =
		feature_metrics::ratio(metrics.trace_complete_count, metrics.trace_required_count);

	Some(metrics)
}

pub(super) fn unsupported_scheduled_outputs_impl(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
) -> Vec<UnsupportedClaimReport> {
	answer
		.scheduled_tasks
		.iter()
		.flat_map(|task| {
			task.outputs.iter().filter_map(|output| {
				if output.evidence_refs.is_empty() {
					return Some(scheduled_unsupported_claim_report(
						job,
						task,
						output,
						"scheduled task output has no evidence refs",
					));
				}
				if scheduled_output_is_unsupported_current(output) {
					return Some(scheduled_unsupported_claim_report(
						job,
						task,
						output,
						"unsupported scheduled task claim is still recommended or marked current",
					));
				}

				None
			})
		})
		.collect()
}

fn accumulate_scheduled_memory_metrics(
	task: &ScheduledMemoryTaskArtifact,
	metrics: &mut ScheduledMemoryJobMetrics,
	task_kinds: &mut BTreeSet<String>,
) {
	metrics.source_trace_selected_count += task.source_trace.selected_source_refs.len();
	metrics.source_trace_dropped_count += task.source_trace.dropped_source_refs.len();
	metrics.source_trace_stale_count += task.source_trace.stale_source_refs.len();
	metrics.source_trace_superseded_count += task.source_trace.superseded_source_refs.len();
	metrics.source_trace_tombstone_count += task.source_trace.tombstone_source_refs.len();
	metrics.trace_required_count += 1;
	metrics.source_mutation_count += task.source_mutations.len()
		+ task.source_mutations.iter().map(forbidden_diff_key_count).sum::<usize>();

	task_kinds.insert(task.task_kind.clone());

	if scheduled_trace_is_complete(task.execution_trace.as_ref()) {
		metrics.trace_complete_count += 1;
	}

	let non_current_refs =
		feature_metrics::memory_summary_non_current_trace_refs(&task.source_trace);
	let tombstone_refs = feature_metrics::proactive_tombstone_trace_refs(&task.source_trace);

	for output in &task.outputs {
		metrics.output_count += 1;
		metrics.evidence_ref_required_count += 1;

		if output.evidence_refs.is_empty() {
			metrics.untraced_output_count += 1;
		} else {
			metrics.evidence_ref_output_count += 1;
		}
		if scheduled_output_has_freshness(output) {
			metrics.freshness_marker_count += 1;
		}
		if scheduled_output_has_action_rationale(output) {
			metrics.action_rationale_count += 1;
		}
		if output.freshness.status == "current" {
			metrics.current_output_count += 1;
		} else {
			metrics.non_current_output_count += 1;
		}
		if scheduled_output_is_invalid_current(output, &non_current_refs) {
			metrics.invalid_current_output_count += 1;
		}
		if scheduled_output_is_unsupported_current(output) {
			metrics.unsupported_current_output_count += 1;
		}
		if scheduled_output_is_tombstone_violation(output, &tombstone_refs) {
			metrics.tombstone_violation_count += 1;
		}
	}
}

fn scheduled_trace_is_complete(trace: Option<&ScheduledMemoryExecutionTrace>) -> bool {
	let Some(trace) = trace else {
		return false;
	};

	trace.status == "completed"
		&& !trace.trace_id.trim().is_empty()
		&& !trace.output_ref.trim().is_empty()
		&& !trace.stages.is_empty()
		&& trace
			.stages
			.iter()
			.any(|stage| stage.stage_name == "output_readback" && !stage.evidence_refs.is_empty())
}

fn scheduled_output_has_freshness(output: &ScheduledMemoryOutput) -> bool {
	if output.freshness.status.trim().is_empty() {
		return false;
	}

	match output.freshness.status.as_str() {
		"superseded" => !output.freshness.superseded_by.is_empty(),
		"tombstoned" => !output.freshness.tombstone_refs.is_empty(),
		_ => true,
	}
}

fn scheduled_output_has_action_rationale(output: &ScheduledMemoryOutput) -> bool {
	!output.action.decision.trim().is_empty()
		&& !output.action.reason_code.trim().is_empty()
		&& !output.action.reason.trim().is_empty()
}

fn scheduled_output_is_invalid_current(
	output: &ScheduledMemoryOutput,
	non_current_refs: &BTreeSet<&str>,
) -> bool {
	output.freshness.status == "current"
		&& (!output.freshness.superseded_by.is_empty()
			|| !output.freshness.tombstone_refs.is_empty()
			|| output
				.evidence_refs
				.iter()
				.any(|evidence_id| non_current_refs.contains(evidence_id.as_str())))
}

fn scheduled_output_is_unsupported_current(output: &ScheduledMemoryOutput) -> bool {
	!output.unsupported_claim_flags.is_empty()
		&& (output.action.decision == "recommend" || output.freshness.status == "current")
}

fn scheduled_output_is_tombstone_violation(
	output: &ScheduledMemoryOutput,
	tombstone_refs: &BTreeSet<&str>,
) -> bool {
	output.freshness.status == "current"
		&& (!output.freshness.tombstone_refs.is_empty()
			|| output
				.evidence_refs
				.iter()
				.any(|evidence_id| tombstone_refs.contains(evidence_id.as_str())))
}

fn scheduled_unsupported_claim_report(
	job: &RealWorldJob,
	task: &ScheduledMemoryTaskArtifact,
	output: &ScheduledMemoryOutput,
	reason: &str,
) -> UnsupportedClaimReport {
	UnsupportedClaimReport {
		suite_id: job.suite.clone(),
		job_id: job.job_id.clone(),
		claim_id: Some(format!("{}:{}", task.task_run_id, output.output_id)),
		claim_text: feature_metrics::bounded_text(output.text.as_str(), 240),
		reason: reason.to_string(),
		evidence_ids: output.evidence_refs.clone(),
	}
}
