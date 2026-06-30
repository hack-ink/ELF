mod candidates;
mod items;
mod preamble;
mod stage_items;
mod stages;
mod traces;

use color_eyre::Result;

use crate::{
	cli::Args,
	rows::{CandidateRow, ItemRow, StageItemRow, StageRow, TraceRow},
};

pub(super) fn render_fixture_sql(
	args: &Args,
	traces: &[TraceRow],
	candidates: &[CandidateRow],
	items: &[ItemRow],
	stages: &[StageRow],
	stage_items: &[StageItemRow],
) -> Result<String> {
	let mut out = String::new();

	preamble::render_preamble(args, &mut out);
	traces::render_traces(&mut out, traces)?;
	candidates::render_candidates(&mut out, candidates)?;
	items::render_items(&mut out, items)?;
	stages::render_stages(&mut out, stages)?;
	stage_items::render_stage_items(&mut out, stage_items)?;

	out.push_str("COMMIT;\n");

	Ok(out)
}
