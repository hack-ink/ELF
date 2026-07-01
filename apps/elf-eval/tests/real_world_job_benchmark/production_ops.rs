mod production_ops_evidence;
mod production_ops_failure_cases;
mod production_ops_jobs;
mod production_ops_summary;

use color_eyre::Result;

use crate::support;

#[test]
fn production_ops_fixtures_report_bounded_typed_states() -> Result<()> {
	let report = support::run_json_report_from(support::production_ops_fixture_dir())?;

	production_ops_summary::assert_production_ops_summary(&report)?;
	production_ops_jobs::assert_production_ops_jobs(&report)?;
	production_ops_evidence::assert_production_ops_operational_evidence(&report)?;

	Ok(())
}
