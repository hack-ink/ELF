use color_eyre::Result;

use crate::{rows::ItemRow, sql};

pub(super) fn render_items(out: &mut String, items: &[ItemRow]) -> Result<()> {
	if items.is_empty() {
		return Ok(());
	}

	out.push_str("INSERT INTO search_trace_items (\n");
	out.push_str("	item_id,\n");
	out.push_str("	trace_id,\n");
	out.push_str("	note_id,\n");
	out.push_str("	chunk_id,\n");
	out.push_str("	rank,\n");
	out.push_str("	final_score,\n");
	out.push_str("	explain\n");
	out.push_str(")\nVALUES\n");

	for (idx, row) in items.iter().enumerate() {
		out.push_str("	(");
		out.push_str(&sql::sql_uuid(&row.item_id));
		out.push_str(", ");
		out.push_str(&sql::sql_uuid(&row.trace_id));
		out.push_str(", ");
		out.push_str(&sql::sql_uuid(&row.note_id));
		out.push_str(", ");
		out.push_str(&sql::sql_opt_uuid(&row.chunk_id));
		out.push_str(", ");
		out.push_str(&row.rank.to_string());
		out.push_str(", ");
		out.push_str(&sql::sql_f32(row.final_score));
		out.push_str(", ");
		out.push_str(&sql::sql_jsonb(&row.explain)?);
		out.push(')');

		if idx + 1 == items.len() {
			out.push_str(";\n\n");
		} else {
			out.push_str(",\n");
		}
	}

	Ok(())
}
