use super::*;

#[test]
fn adversarial_quality_fixture_catches_unsupported_and_stale_regressions() -> Result<()> {
	let temp_dir =
		env::temp_dir().join(format!("elf-adversarial-quality-regression-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	assert_stale_regression_is_wrong_result(&temp_dir)?;
	assert_unsupported_regression_is_unsupported_claim(&temp_dir)?;

	Ok(())
}

fn assert_stale_regression_is_wrong_result(temp_dir: &Path) -> Result<()> {
	let stale_fixture = adversarial_quality_fixture_dir().join("stale_fact_current_answer.json");
	let mut stale = load_json(&stale_fixture)?;

	set_json_pointer(
		&mut stale,
		"/corpus/adapter_response/answer/content",
		Value::String(
			"Run cargo make check before review handoff because that is the current gate."
				.to_string(),
		),
	)?;
	set_json_pointer(
		&mut stale,
		"/corpus/adapter_response/answer/evidence_ids",
		serde_json::json!(["stale-ops-runbook-v1"]),
	)?;
	set_json_pointer(
		&mut stale,
		"/corpus/adapter_response/answer/claims",
		serde_json::json!([
			{
				"claim_id": "current_gate_sequence",
				"text": "Run cargo make check before review handoff.",
				"evidence_ids": ["stale-ops-runbook-v1"],
				"confidence": "high"
			}
		]),
	)?;

	fs::write(temp_dir.join("stale_regression.json"), serde_json::to_vec_pretty(&stale)?)?;

	let stale_report = run_json_report_from(temp_dir.to_path_buf())?;
	let stale_jobs = array_at(&stale_report, "/jobs")?;
	let stale_job =
		find_by_field(stale_jobs, "/job_id", "adversarial-quality-stale-fact-current-answer-001")?;

	assert_eq!(stale_job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(stale_job.pointer("/stale_answer_count").and_then(Value::as_u64), Some(1));
	assert_eq!(
		stale_report.pointer("/scoreboard/summary_claim").and_then(Value::as_str),
		Some("typed_non_pass_present")
	);
	assert_eq!(
		stale_report.pointer("/scoreboard/job_summary_claim").and_then(Value::as_str),
		Some("typed_non_pass_present")
	);
	assert_eq!(
		stale_report.pointer("/scoreboard/job_typed_non_pass_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		stale_report.pointer("/scoreboard/typed_non_pass_count").and_then(Value::as_u64),
		Some(241)
	);
	assert!(array_contains_str(
		&stale_report,
		"/scoreboard/typed_non_pass_states_present",
		"wrong_result"
	)?);
	assert!(array_contains_str(
		&stale_report,
		"/scoreboard/job_typed_non_pass_states_present",
		"wrong_result"
	)?);

	fs::remove_file(temp_dir.join("stale_regression.json"))?;

	Ok(())
}

fn assert_unsupported_regression_is_unsupported_claim(temp_dir: &Path) -> Result<()> {
	let unsupported_fixture =
		adversarial_quality_fixture_dir().join("unsupported_claim_refusal.json");
	let mut unsupported = load_json(&unsupported_fixture)?;

	set_json_pointer(
		&mut unsupported,
		"/corpus/adapter_response/answer/content",
		Value::String(
			"The fixture proves private-corpus production quality and broad competitor superiority."
				.to_string(),
		),
	)?;
	set_json_pointer(
		&mut unsupported,
		"/corpus/adapter_response/answer/evidence_ids",
		serde_json::json!(["unsupported-production-quality-trap"]),
	)?;
	set_json_pointer(
		&mut unsupported,
		"/corpus/adapter_response/answer/claims",
		serde_json::json!([
			{
				"claim_id": "production_quality_proven",
				"text": "The fixture proves private-corpus production quality and broad competitor superiority.",
				"evidence_ids": ["unsupported-production-quality-trap"],
				"confidence": "high"
			}
		]),
	)?;

	fs::write(
		temp_dir.join("unsupported_regression.json"),
		serde_json::to_vec_pretty(&unsupported)?,
	)?;

	let unsupported_report = run_json_report_from(temp_dir.to_path_buf())?;
	let unsupported_jobs = array_at(&unsupported_report, "/jobs")?;
	let unsupported_job = find_by_field(
		unsupported_jobs,
		"/job_id",
		"adversarial-quality-unsupported-claim-refusal-001",
	)?;

	assert_eq!(
		unsupported_job.pointer("/status").and_then(Value::as_str),
		Some("unsupported_claim")
	);
	assert_eq!(
		unsupported_report.pointer("/summary/unsupported_claim").and_then(Value::as_u64),
		Some(1)
	);
	assert!(array_contains_str(
		&unsupported_report,
		"/scoreboard/typed_non_pass_states_present",
		"unsupported_claim"
	)?);
	assert!(array_contains_str(
		&unsupported_report,
		"/scoreboard/job_typed_non_pass_states_present",
		"unsupported_claim"
	)?);

	Ok(())
}

#[test]
fn adversarial_quality_repeated_fixture_run_is_deterministic() -> Result<()> {
	let first = run_json_report_from(adversarial_quality_fixture_dir())?;
	let second = run_json_report_from(adversarial_quality_fixture_dir())?;

	assert_eq!(first.pointer("/scoreboard"), second.pointer("/scoreboard"));
	assert_eq!(first.pointer("/summary"), second.pointer("/summary"));
	assert_eq!(first.pointer("/suites"), second.pointer("/suites"));
	assert_eq!(first.pointer("/jobs"), second.pointer("/jobs"));

	Ok(())
}
