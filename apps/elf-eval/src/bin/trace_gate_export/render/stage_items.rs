use color_eyre::Result;

use crate::{rows::StageItemRow, sql};

pub(super) fn render_stage_items(out: &mut String, stage_items: &[StageItemRow]) -> Result<()> {
	if stage_items.is_empty() {
		return Ok(());
	}

	out.push_str("INSERT INTO search_trace_stage_items (\n");
	out.push_str("	id,\n");
	out.push_str("	stage_id,\n");
	out.push_str("	item_id,\n");
	out.push_str("	note_id,\n");
	out.push_str("	chunk_id,\n");
	out.push_str("	metrics\n");
	out.push_str(")\nVALUES\n");

	for (idx, row) in stage_items.iter().enumerate() {
		out.push_str("	(");
		out.push_str(&sql::sql_uuid(&row.id));
		out.push_str(", ");
		out.push_str(&sql::sql_uuid(&row.stage_id));
		out.push_str(", ");
		out.push_str(&sql::sql_opt_uuid(&row.item_id));
		out.push_str(", ");
		out.push_str(&sql::sql_opt_uuid(&row.note_id));
		out.push_str(", ");
		out.push_str(&sql::sql_opt_uuid(&row.chunk_id));
		out.push_str(", ");
		out.push_str(&sql::sql_jsonb(&row.metrics)?);
		out.push(')');

		if idx + 1 == stage_items.len() {
			out.push_str(";\n\n");
		} else {
			out.push_str(",\n");
		}
	}

	Ok(())
}
