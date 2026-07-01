use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn runner_discovers_nested_fixture_layout() -> Result<()> {
	let report = support::run_json_report_from(support::fixture_root())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(82));

	Ok(())
}
