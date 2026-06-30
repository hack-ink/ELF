use std::fs;

use color_eyre::{Result, eyre};
use serde_json::Value;

use crate::support;

#[test]
fn service_native_dreaming_readback_report_materializes_public_jobs() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::service_native_dreaming_readback_report_json_path()?,
	)?)?;
	let materialization = serde_json::from_str::<Value>(&fs::read_to_string(
		support::service_native_dreaming_readback_materialization_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(support::service_native_dreaming_readback_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_service_native_dreaming_report_summary(&report)?;
	assert_service_native_dreaming_report_jobs(&report)?;
	assert_service_native_dreaming_materialization(&materialization)?;
	assert_service_native_dreaming_docs(&markdown, &benchmarking_index, &readme);

	Ok(())
}

fn assert_service_native_dreaming_report_summary(report: &Value) -> Result<()> {
	assert_eq!(
		report.pointer("/adapter/adapter_id").and_then(Value::as_str),
		Some("elf_service_native_dreaming")
	);
	assert_eq!(
		report.pointer("/adapter/behavior").and_then(Value::as_str),
		Some("service_native_dreaming_readback")
	);
	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(11));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(9));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/source_ref_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/quote_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(
		report.pointer("/summary/memory_summary/source_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/evidence_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/trace_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/source_mutation_count").and_then(Value::as_u64),
		Some(0)
	);

	let suites = support::array_at(report, "/suites")?;
	let memory = support::find_by_field(suites, "/suite_id", "memory_summary")?;
	let proactive = support::find_by_field(suites, "/suite_id", "proactive_brief")?;
	let scheduled = support::find_by_field(suites, "/suite_id", "scheduled_memory")?;

	assert_eq!(memory.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(proactive.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(scheduled.pointer("/status").and_then(Value::as_str), Some("blocked"));

	Ok(())
}

fn assert_service_native_dreaming_report_jobs(report: &Value) -> Result<()> {
	let jobs = support::array_at(report, "/jobs")?;
	let memory = support::find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;
	let daily = support::find_by_field(jobs, "/job_id", "proactive-daily-project-brief-001")?;
	let private_brief =
		support::find_by_field(jobs, "/job_id", "proactive-private-corpus-refresh-blocked-001")?;
	let weekly =
		support::find_by_field(jobs, "/job_id", "scheduled-weekly-project-status-summary-001")?;
	let private_scheduled = support::find_by_field(
		jobs,
		"/job_id",
		"scheduled-private-provider-scheduler-blocked-001",
	)?;

	assert_eq!(memory.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(daily.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(weekly.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(private_brief.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(private_scheduled.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert!(!support::array_contains_str(memory, "/produced_evidence", "stale-summary-gap")?);
	assert!(!support::array_contains_str(memory, "/produced_evidence", "summary-temporary-claim")?);
	assert!(!support::array_contains_str(daily, "/produced_evidence", "daily-old-parity-trap")?);
	assert!(!support::array_contains_str(
		weekly,
		"/produced_evidence",
		"scheduled-weekly-hosted-parity-trap"
	)?);

	Ok(())
}

fn assert_service_native_dreaming_materialization(materialization: &Value) -> Result<()> {
	assert_eq!(
		materialization.pointer("/schema").and_then(Value::as_str),
		Some("elf.real_world_live_adapter_materialization/v1")
	);
	assert_eq!(
		materialization.pointer("/adapter_id").and_then(Value::as_str),
		Some("elf_service_native_dreaming")
	);
	assert_eq!(materialization.pointer("/status").and_then(Value::as_str), Some("blocked"));

	let jobs = support::array_at(materialization, "/jobs")?;
	let memory = support::find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;
	let daily = support::find_by_field(jobs, "/job_id", "proactive-daily-project-brief-001")?;
	let private_brief =
		support::find_by_field(jobs, "/job_id", "proactive-private-corpus-refresh-blocked-001")?;

	for job in jobs {
		match job.pointer("/status").and_then(Value::as_str) {
			Some("pass") => {
				assert_eq!(
					job.pointer("/dreaming_readback/runtime_path").and_then(Value::as_str),
					Some("ElfService::add_note -> ElfService::list -> derived readback artifact")
				);
				assert!(
					support::array_at(job, "/dreaming_readback/missing_source_refs")?.is_empty()
				);
				assert_eq!(
					job.pointer("/dreaming_readback/source_mutation_count").and_then(Value::as_u64),
					Some(0)
				);
				assert_eq!(
					job.pointer("/dreaming_readback/no_source_mutation_checked")
						.and_then(Value::as_bool),
					Some(true)
				);
			},
			Some("blocked") => {
				assert!(job.pointer("/dreaming_readback").is_none_or(Value::is_null));
			},
			status => {
				return Err(eyre::eyre!(
					"unexpected service-native materialization status: {status:?}"
				));
			},
		}
	}

	assert!(support::array_contains_str(
		memory,
		"/dreaming_readback/selected_source_refs",
		"stale-summary-gap"
	)?);
	assert!(!support::array_contains_str(memory, "/evidence_ids", "stale-summary-gap")?);
	assert!(support::array_contains_str(
		daily,
		"/dreaming_readback/selected_source_refs",
		"daily-old-parity-trap"
	)?);
	assert!(!support::array_contains_str(daily, "/evidence_ids", "daily-old-parity-trap")?);
	assert!(private_brief.pointer("/dreaming_readback").is_none_or(Value::is_null));

	Ok(())
}

fn assert_service_native_dreaming_docs(markdown: &str, benchmarking_index: &str, readme: &str) {
	assert!(markdown.contains("9 pass"));
	assert!(markdown.contains("0 wrong_result"));
	assert!(markdown.contains("2 typed blocked"));
	assert!(markdown.contains("ElfService::add_note -> ElfService::list"));
	assert!(markdown.contains("Do not claim ELF broadly beats OpenAI Pulse"));
	assert!(benchmarking_index.contains("2026-06-19-service-native-dreaming-readback-report.md"));
	assert!(readme.contains("Service-native Dreaming readback after XY-986"));
	assert!(readme.contains("real-world-memory-service-native-dreaming"));
}
