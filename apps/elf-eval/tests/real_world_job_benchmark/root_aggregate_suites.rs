use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(crate) fn assert_root_aggregate_suites(report: &Value) -> Result<()> {
	let suites = support::array_at(report, "/suites")?;

	for suite_id in [
		"trust_source_of_truth",
		"work_resume",
		"project_decisions",
		"retrieval",
		"capture_integration",
		"personalization",
		"consolidation",
		"memory_summary",
		"knowledge_compilation",
		"operator_debugging_ux",
		"memory_evolution",
		"adversarial_quality",
		"core_archival_memory",
		"work_continuity",
	] {
		let suite = support::find_by_field(suites, "/suite_id", suite_id)?;

		assert_eq!(suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	}

	let memory_evolution = support::find_by_field(suites, "/suite_id", "memory_evolution")?;

	assert_eq!(memory_evolution.pointer("/status").and_then(Value::as_str), Some("pass"));

	let project_decisions = support::find_by_field(suites, "/suite_id", "project_decisions")?;

	assert_eq!(project_decisions.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(
		project_decisions.pointer("/update_rationale_available_count").and_then(Value::as_u64),
		Some(5)
	);

	let debug_suite = support::find_by_field(suites, "/suite_id", "operator_debugging_ux")?;

	assert_eq!(debug_suite.pointer("/status").and_then(Value::as_str), Some("pass"));

	let core_suite = support::find_by_field(suites, "/suite_id", "core_archival_memory")?;

	assert_eq!(core_suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(core_suite.pointer("/encoded_job_count").and_then(Value::as_u64), Some(6));

	let adversarial = support::find_by_field(suites, "/suite_id", "adversarial_quality")?;

	assert_eq!(adversarial.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(adversarial.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));

	let production_ops = support::find_by_field(suites, "/suite_id", "production_ops")?;

	assert_eq!(production_ops.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(production_ops.pointer("/encoded_job_count").and_then(Value::as_u64), Some(8));

	let proactive = support::find_by_field(suites, "/suite_id", "proactive_brief")?;

	assert_eq!(proactive.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(proactive.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));

	let scheduled = support::find_by_field(suites, "/suite_id", "scheduled_memory")?;

	assert_eq!(scheduled.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(scheduled.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));

	let source_library = support::find_by_field(suites, "/suite_id", "source_library")?;

	assert_eq!(source_library.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(source_library.pointer("/encoded_job_count").and_then(Value::as_u64), Some(2));

	let context_trajectory = support::find_by_field(suites, "/suite_id", "context_trajectory")?;

	assert_eq!(context_trajectory.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(context_trajectory.pointer("/encoded_job_count").and_then(Value::as_u64), Some(3));

	let work_continuity = support::find_by_field(suites, "/suite_id", "work_continuity")?;

	assert_eq!(work_continuity.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(work_continuity.pointer("/encoded_job_count").and_then(Value::as_u64), Some(8));

	Ok(())
}
