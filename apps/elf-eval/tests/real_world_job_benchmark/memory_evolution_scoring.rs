use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn memory_evolution_fixtures_report_temporal_and_staleness_metrics() -> Result<()> {
	let report = support::run_json_report_from(support::evolution_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/encoded_suite_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/not_encoded").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/stale_answer_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/conflict_detection_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/update_rationale_available_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/temporal_validity_not_encoded_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/history_readback_encoded_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/evolution/temporal_validity_not_encoded_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/evolution/history_readback_encoded_count").and_then(Value::as_u64),
		Some(1)
	);

	let suites = support::array_at(&report, "/suites")?;
	let memory_evolution = support::find_by_field(suites, "/suite_id", "memory_evolution")?;

	assert_eq!(memory_evolution.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		memory_evolution.pointer("/temporal_validity_not_encoded_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		memory_evolution.pointer("/history_readback_encoded_count").and_then(Value::as_u64),
		Some(1)
	);

	let jobs = support::array_at(&report, "/jobs")?;
	let preference_job =
		support::find_by_field(jobs, "/job_id", "memory-evolution-preference-001")?;
	let relation_job =
		support::find_by_field(jobs, "/job_id", "memory-evolution-relation-temporal-001")?;

	assert_eq!(
		preference_job.pointer("/evolution/history_readback_encoded").and_then(Value::as_bool),
		Some(true)
	);
	assert!(support::array_contains_str(preference_job, "/evolution/history_event_types", "add")?);
	assert!(support::array_contains_str(
		preference_job,
		"/evolution/history_event_types",
		"update"
	)?);
	assert!(support::array_contains_str(
		preference_job,
		"/evolution/history_event_types",
		"ignore"
	)?);
	assert_eq!(
		preference_job
			.pointer("/evolution/history_requires_note_version_links")
			.and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		preference_job.pointer("/evolution/selected_current_evidence/0").and_then(Value::as_str),
		Some("pref-current-concise-rationale")
	);
	assert_eq!(
		preference_job.pointer("/evolution/selected_historical_evidence/0").and_then(Value::as_str),
		Some("pref-old-terse-bullets")
	);
	assert_eq!(
		preference_job.pointer("/evolution/selected_rationale_evidence/0").and_then(Value::as_str),
		Some("pref-update-rationale")
	);
	assert_eq!(relation_job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		relation_job.pointer("/evolution/temporal_validity_not_encoded").and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		relation_job.pointer("/evolution/temporal_validity_encoded").and_then(Value::as_bool),
		Some(true)
	);

	let follow_ups = support::array_at(&report, "/follow_ups")?;

	assert!(follow_ups.is_empty());

	Ok(())
}
