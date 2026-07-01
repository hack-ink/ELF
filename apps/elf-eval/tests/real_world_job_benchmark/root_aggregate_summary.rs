mod root_aggregate_summary_counts;
mod root_aggregate_summary_scoreboard;
mod root_aggregate_summary_suite_summaries;

use color_eyre::Result;
use serde_json::Value;

pub(crate) fn assert_root_aggregate_summary(report: &Value) -> Result<()> {
	root_aggregate_summary_counts::assert_root_summary_counts(report);
	root_aggregate_summary_scoreboard::assert_root_scoreboard_summary(report)?;
	root_aggregate_summary_counts::assert_root_consolidation_summary(report);
	root_aggregate_summary_suite_summaries::assert_root_knowledge_summary(report);
	root_aggregate_summary_suite_summaries::assert_root_proactive_brief_summary(report);
	root_aggregate_summary_suite_summaries::assert_root_scheduled_memory_summary(report);
	root_aggregate_summary_suite_summaries::assert_root_work_continuity_summary(report);

	Ok(())
}
