use crate::app::trace_compare::types::{TraceCompareStageDelta, TraceCompareStageRow};

pub(in crate::app::trace_compare) fn build_trace_compare_stage_deltas(
	stage_rows: &[TraceCompareStageRow],
	a_selected_count: u32,
	b_selected_count: u32,
) -> Vec<TraceCompareStageDelta> {
	if stage_rows.is_empty() {
		return vec![TraceCompareStageDelta {
			stage_order: 1,
			stage_name: "selection.final".to_string(),
			baseline_item_count: 0,
			a_item_count: a_selected_count,
			b_item_count: b_selected_count,
			item_count_delta: b_selected_count as i64 - a_selected_count as i64,
			baseline_stats: None,
		}];
	}

	let mut out = Vec::with_capacity(stage_rows.len());

	for row in stage_rows {
		let baseline_item_count = row.item_count.max(0) as u32;
		let (a_item_count, b_item_count) = if row.stage_name == "selection.final" {
			(a_selected_count, b_selected_count)
		} else {
			(baseline_item_count, baseline_item_count)
		};
		let baseline_stats = row.stage_payload.get("stats").cloned();

		out.push(TraceCompareStageDelta {
			stage_order: row.stage_order.max(0) as u32,
			stage_name: row.stage_name.clone(),
			baseline_item_count,
			a_item_count,
			b_item_count,
			item_count_delta: b_item_count as i64 - a_item_count as i64,
			baseline_stats,
		});
	}

	out
}
