use super::*;

pub(super) fn aggregate_job_report_state(job_reports: &[JobReport]) -> String {
	if job_reports.is_empty() {
		return "not_tested".to_string();
	}

	let refs = job_reports.iter().collect::<Vec<_>>();

	scoreboard_result_state(aggregate_status(&refs)).to_string()
}

pub(super) fn jobs_have_tag(jobs: &[RealWorldJob], tag: &str) -> bool {
	!jobs.is_empty() && jobs.iter().all(|job| job.tags.iter().any(|candidate| candidate == tag))
}

pub(super) fn scoreboard_mean_metric(sum: f64, count: usize) -> f64 {
	if count == 0 { 1.0 } else { round3(sum / count as f64) }
}

pub(super) fn scoreboard_is_update_job(job: &RealWorldJob) -> bool {
	scoreboard_has_any_tag(
		job,
		&["update", "correction_persistence", "current_authority", "conflicting_source_authority"],
	)
}

pub(super) fn scoreboard_is_delete_job(job: &RealWorldJob) -> bool {
	scoreboard_has_any_tag(job, &["delete", "ttl", "tombstone"])
}

pub(super) fn scoreboard_is_rollback_history_job(job: &RealWorldJob) -> bool {
	scoreboard_has_any_tag(job, &["rollback", "correction_persistence"])
}

pub(super) fn scoreboard_has_any_tag(job: &RealWorldJob, tags: &[&str]) -> bool {
	job.tags.iter().any(|tag| tags.contains(&tag.as_str()))
}

pub(super) fn scoreboard_apply_comparability_gaps(row: &mut ScoreboardRow) {
	if !row.same_corpus {
		row.next_evidence.push("Map this product to the same corpus.".to_string());
	}
	if !row.source_id_mapped {
		row.next_evidence.push("Map returned evidence to stable source ids.".to_string());
	}
	if !row.held_out {
		row.next_evidence.push("Publish a held-out split for this row.".to_string());
	}
	if !row.leakage_audited {
		row.next_evidence.push("Publish leakage-audit evidence for this row.".to_string());
	}
	if !row.product_runtime {
		row.next_evidence
			.push("Run a Docker-contained product-runtime adapter for this row.".to_string());
	}
	if !row.container_digest_identified {
		row.next_evidence.push("Record container image digest evidence.".to_string());
	}
	if row.result_state != "pass" {
		row.next_evidence
			.push("Resolve typed non-pass state before claiming a comparable pass.".to_string());
	}

	row.comparable = row.same_corpus
		&& row.source_id_mapped
		&& row.held_out
		&& row.leakage_audited
		&& row.product_runtime
		&& row.container_digest_identified
		&& row.result_state == "pass"
		&& row.metrics.retrieval.recall_at_k.is_some()
		&& row.metrics.retrieval.precision_at_k.is_some()
		&& row.metrics.retrieval.mrr.is_some()
		&& row.metrics.retrieval.ndcg.is_some();

	if !row.comparable && row.result_state == "pass" {
		row.result_state = "not_comparable".to_string();
	}
	if !row.comparable {
		row.weaknesses
			.push("This row is not a comparable product-runtime scoreboard pass.".to_string());
	}
}

pub(super) fn scoreboard_optimization_roadmap() -> Vec<String> {
	vec![
		"Capture Docker image digests and runtime metadata for product-runtime rows.".to_string(),
		"Add held-out and leakage-audit manifests before broad competitor comparisons.".to_string(),
		"Promote external adapters from typed blockers to same-corpus source-id-mapped runtime rows only after they emit comparable evidence.".to_string(),
		"Use row-level metrics for optimization direction; do not claim a universal leaderboard.".to_string(),
	]
}

pub(super) fn typed_non_pass_states_present(jobs: &[JobReport]) -> Vec<String> {
	let mut states = BTreeSet::new();

	for job in jobs.iter().filter(|job| job.status != TypedStatus::Pass) {
		states.insert(scoreboard_result_state(job.status).to_string());
	}

	states.into_iter().collect()
}

pub(super) fn external_typed_non_pass_count(summary: &ExternalAdapterSummary) -> usize {
	[
		&summary.overall_status_counts,
		&summary.capability_status_counts,
		&summary.suite_status_counts,
		&summary.scenario_status_counts,
	]
	.into_iter()
	.map(scoreboard_adapter_typed_non_pass_count)
	.sum::<usize>()
		+ summary.scenario_outcome_counts.not_tested
}

fn scoreboard_adapter_typed_non_pass_count(counts: &AdapterStatusCounts) -> usize {
	counts.blocked
		+ counts.incomplete
		+ counts.wrong_result
		+ counts.lifecycle_fail
		+ counts.not_encoded
		+ counts.unsupported
}

pub(super) fn external_typed_non_pass_states_present(
	summary: &ExternalAdapterSummary,
) -> Vec<String> {
	let mut states = BTreeSet::new();

	for counts in [
		&summary.overall_status_counts,
		&summary.capability_status_counts,
		&summary.suite_status_counts,
		&summary.scenario_status_counts,
	] {
		if counts.blocked > 0 {
			states.insert("blocked".to_string());
		}
		if counts.incomplete > 0 {
			states.insert("incomplete".to_string());
		}
		if counts.wrong_result + counts.lifecycle_fail > 0 {
			states.insert("wrong_result".to_string());
		}
		if counts.not_encoded + counts.unsupported > 0 {
			states.insert("not_encoded".to_string());
		}
	}

	if summary.scenario_outcome_counts.not_tested > 0 {
		states.insert("not_tested".to_string());
	}

	states.into_iter().collect()
}

pub(super) fn scoreboard_result_state(status: TypedStatus) -> &'static str {
	match status {
		TypedStatus::Pass => "pass",
		TypedStatus::WrongResult | TypedStatus::LifecycleFail => "wrong_result",
		TypedStatus::Incomplete => "incomplete",
		TypedStatus::Blocked => "blocked",
		TypedStatus::NotEncoded => "not_encoded",
		TypedStatus::UnsupportedClaim => "unsupported_claim",
	}
}

pub(super) fn scoreboard_evidence_class_counts(
	external_adapters: &ExternalAdapterSection,
) -> BTreeMap<String, usize> {
	let mut counts = SCOREBOARD_EVIDENCE_CLASSES
		.iter()
		.map(|state| (state.to_string(), 0))
		.collect::<BTreeMap<_, _>>();

	for adapter in &external_adapters.adapters {
		let state = scoreboard_evidence_class(adapter.evidence_class.as_str());

		*counts.entry(state.to_string()).or_insert(0) += 1;
	}

	counts
}

pub(super) fn scoreboard_evidence_class(evidence_class: &str) -> &str {
	match evidence_class {
		"live_baseline_only" => "live_baseline",
		other => other,
	}
}

pub(super) fn scoreboard_summary_claim(
	jobs: &[JobReport],
	typed_non_pass_count: usize,
) -> &'static str {
	if jobs.is_empty() {
		"not_tested"
	} else if typed_non_pass_count > 0 {
		"typed_non_pass_present"
	} else {
		"all_encoded_jobs_passed"
	}
}
