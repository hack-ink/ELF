use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn memory_authority_benchmark_covers_entity_history_and_core_archive_strengths() -> Result<()> {
	let report = support::run_json_report_from(support::real_world_memory_fixture_dir())?;

	assert_eq!(
		report.pointer("/summary/history_readback_encoded_count").and_then(Value::as_u64),
		Some(4)
	);

	let suites = support::array_at(&report, "/suites")?;
	let memory_evolution = support::find_by_field(suites, "/suite_id", "memory_evolution")?;
	let core_archival = support::find_by_field(suites, "/suite_id", "core_archival_memory")?;

	assert_eq!(memory_evolution.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(core_archival.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		memory_evolution.pointer("/history_readback_encoded_count").and_then(Value::as_u64),
		Some(3)
	);
	assert_eq!(core_archival.pointer("/encoded_job_count").and_then(Value::as_u64), Some(6));

	let jobs = support::array_at(&report, "/jobs")?;
	let preference = support::find_by_field(jobs, "/job_id", "memory-evolution-preference-001")?;
	let core_attachment =
		support::find_by_field(jobs, "/job_id", "core-archival-core-block-attachment-001")?;
	let archival_fallback =
		support::find_by_field(jobs, "/job_id", "core-archival-archival-fallback-001")?;

	assert_eq!(preference.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		preference.pointer("/evolution/history_readback_encoded").and_then(Value::as_bool),
		Some(true)
	);
	assert!(support::array_contains_str(preference, "/evolution/history_event_types", "update")?);
	assert_eq!(core_attachment.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(archival_fallback.pointer("/status").and_then(Value::as_str), Some("pass"));

	let adapters = support::array_at(&report, "/external_adapters/adapters")?;
	let mem0 = support::find_by_field(adapters, "/adapter_id", "mem0_openmemory_live_baseline")?;
	let letta = support::find_by_field(adapters, "/adapter_id", "letta_research_gate")?;
	let mem0_scenarios = support::array_at(mem0, "/scenarios")?;
	let mem0_history =
		support::find_by_field(mem0_scenarios, "/scenario_id", "preference_correction_history")?;
	let mem0_entity =
		support::find_by_field(mem0_scenarios, "/scenario_id", "entity_scoped_personalization")?;

	assert_eq!(mem0_history.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(mem0_entity.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(mem0_history.pointer("/comparison_outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(mem0_entity.pointer("/comparison_outcome").and_then(Value::as_str), Some("tie"));

	let letta_scenarios = support::array_at(letta, "/scenarios")?;
	let letta_core =
		support::find_by_field(letta_scenarios, "/scenario_id", "core_block_attachment_readback")?;
	let letta_fallback =
		support::find_by_field(letta_scenarios, "/scenario_id", "archival_fallback_readback")?;

	for scenario in [letta_core, letta_fallback] {
		assert_eq!(
			scenario.pointer("/suite_id").and_then(Value::as_str),
			Some("core_archival_memory")
		);
		assert_eq!(scenario.pointer("/status").and_then(Value::as_str), Some("blocked"));
		assert_eq!(
			scenario.pointer("/comparison_outcome").and_then(Value::as_str),
			Some("blocked")
		);
	}

	Ok(())
}
