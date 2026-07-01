use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(super) fn assert_production_ops_summary(report: &Value) -> Result<()> {
	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(8));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/incomplete").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/not_encoded").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(
		report.pointer("/summary/qdrant_rebuild_case_count").and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report.pointer("/private_corpus_redaction/private_fixture_count").and_then(Value::as_u64),
		Some(1)
	);

	let suites = support::array_at(report, "/suites")?;
	let production_ops = support::find_by_field(suites, "/suite_id", "production_ops")?;

	assert_eq!(production_ops.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(production_ops.pointer("/encoded_job_count").and_then(Value::as_u64), Some(8));

	Ok(())
}
