mod baseline_counts;
mod header_shape;
mod markdown;

use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn dreaming_readiness_stage_ledger_preserves_gate_shape() -> Result<()> {
	let ledger = serde_json::from_str::<Value>(&fs::read_to_string(
		support::dreaming_readiness_stage_ledger_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(support::dreaming_readiness_stage_ledger_markdown_path()?)?;
	let stages = support::array_at(&ledger, "/stage_gates")?;

	header_shape::assert_dreaming_readiness_ledger_header(&ledger)?;
	header_shape::assert_dreaming_readiness_stage_shape(&ledger, stages)?;
	baseline_counts::assert_dreaming_readiness_baseline_counts(&ledger, stages)?;
	markdown::assert_dreaming_readiness_markdown_boundaries(&markdown);

	Ok(())
}
