#[path = "root_aggregate_jobs.rs"] mod jobs;
#[path = "root_aggregate_suites.rs"] mod suites;
#[path = "root_aggregate_summary.rs"] mod summary;

use color_eyre::Result;

use crate::support;

#[test]
fn real_world_memory_fixtures_report_aggregate_metrics() -> Result<()> {
	let report = support::run_json_report_from(support::real_world_memory_fixture_dir())?;

	summary::assert_root_aggregate_summary(&report)?;
	suites::assert_root_aggregate_suites(&report)?;
	jobs::assert_root_aggregate_jobs(&report)?;

	Ok(())
}
