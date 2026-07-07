#[path = "root_aggregate_summary/counts.rs"] mod counts;
#[path = "root_aggregate_summary/scoreboard.rs"] mod scoreboard;
#[path = "root_aggregate_summary/suite_summaries.rs"] mod suite_summaries;

use color_eyre::Result;
use serde_json::Value;

pub(crate) fn assert_root_aggregate_summary(report: &Value) -> Result<()> {
	counts::assert_root_summary_counts(report);
	scoreboard::assert_root_scoreboard_summary(report)?;
	counts::assert_root_consolidation_summary(report);
	counts::assert_root_local_organizer_summary(report);
	suite_summaries::assert_root_knowledge_summary(report);
	suite_summaries::assert_root_proactive_brief_summary(report);
	suite_summaries::assert_root_scheduled_memory_summary(report);
	suite_summaries::assert_root_work_continuity_summary(report);

	Ok(())
}
