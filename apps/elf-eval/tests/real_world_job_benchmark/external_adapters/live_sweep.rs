use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(super) fn assert_live_sweep_record(adapter: &Value, production_ops_status: &str) -> Result<()> {
	let suites = support::array_at(adapter, "/suites")?;
	let capabilities = support::array_at(adapter, "/capabilities")?;
	let adapter_id = adapter.pointer("/adapter_id").and_then(Value::as_str).unwrap_or_default();
	let targeted = support::find_by_field(capabilities, "/capability", "targeted_live_pass")?;
	let full_pass = support::find_by_field(capabilities, "/capability", "full_suite_live_pass")?;
	let work_resume = support::find_by_field(suites, "/suite_id", "work_resume")?;
	let memory_evolution = support::find_by_field(suites, "/suite_id", "memory_evolution")?;
	let production_ops = support::find_by_field(suites, "/suite_id", "production_ops")?;
	let consolidation = support::find_by_field(suites, "/suite_id", "consolidation")?;
	let knowledge = support::find_by_field(suites, "/suite_id", "knowledge_compilation")?;
	let operator_debug = support::find_by_field(suites, "/suite_id", "operator_debugging_ux")?;
	let capture = support::find_by_field(suites, "/suite_id", "capture_integration")?;
	let personalization = support::find_by_field(suites, "/suite_id", "personalization")?;
	let core_archival = support::find_by_field(suites, "/suite_id", "core_archival_memory")?;
	let context_trajectory = support::find_by_field(suites, "/suite_id", "context_trajectory")?;
	let trust_sot = support::find_by_field(suites, "/suite_id", "trust_source_of_truth")?;
	let retrieval = support::find_by_field(suites, "/suite_id", "retrieval")?;
	let project_decisions = support::find_by_field(suites, "/suite_id", "project_decisions")?;

	assert_eq!(suites.len(), 13);
	assert_eq!(targeted.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(full_pass.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert!(
		adapter
			.pointer("/result/evidence")
			.and_then(Value::as_str)
			.is_some_and(|evidence| evidence.contains("55 jobs across all 13 checked-in suites"))
	);
	assert_eq!(trust_sot.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(work_resume.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(retrieval.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(project_decisions.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(memory_evolution.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		production_ops.pointer("/status").and_then(Value::as_str),
		Some(production_ops_status)
	);

	if adapter_id == "elf_live_real_world" {
		assert_eq!(consolidation.pointer("/status").and_then(Value::as_str), Some("pass"));
		assert_eq!(knowledge.pointer("/status").and_then(Value::as_str), Some("pass"));
		assert_eq!(operator_debug.pointer("/status").and_then(Value::as_str), Some("pass"));
		assert_eq!(capture.pointer("/status").and_then(Value::as_str), Some("pass"));
		assert!(
			capture
				.pointer("/evidence")
				.and_then(Value::as_str)
				.is_some_and(|evidence| evidence.contains("4/4 capture_integration jobs"))
		);
	} else {
		assert_eq!(consolidation.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
		assert_eq!(knowledge.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
		assert_eq!(operator_debug.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
		assert_eq!(capture.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
	}

	assert_eq!(personalization.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(core_archival.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(context_trajectory.pointer("/status").and_then(Value::as_str), Some("blocked"));

	Ok(())
}
