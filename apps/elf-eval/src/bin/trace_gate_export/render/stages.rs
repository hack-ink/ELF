use color_eyre::Result;

use crate::{rows::StageRow, sql};

pub(super) fn render_stages(out: &mut String, stages: &[StageRow]) -> Result<()> {
	if stages.is_empty() {
		return Ok(());
	}

	out.push_str("INSERT INTO search_trace_stages (\n");
	out.push_str("	stage_id,\n");
	out.push_str("	trace_id,\n");
	out.push_str("	stage_order,\n");
	out.push_str("	stage_name,\n");
	out.push_str("	stage_payload,\n");
	out.push_str("	created_at\n");
	out.push_str(")\nVALUES\n");

	for (idx, row) in stages.iter().enumerate() {
		out.push_str("	(");
		out.push_str(&sql::sql_uuid(&row.stage_id));
		out.push_str(", ");
		out.push_str(&sql::sql_uuid(&row.trace_id));
		out.push_str(", ");
		out.push_str(&row.stage_order.to_string());
		out.push_str(", ");
		out.push_str(&sql::sql_text(&row.stage_name));
		out.push_str(", ");
		out.push_str(&sql::sql_jsonb(&row.stage_payload)?);
		out.push_str(", ");
		out.push_str(&sql::sql_timestamptz(&row.created_at)?);
		out.push(')');

		if idx + 1 == stages.len() {
			out.push_str(";\n\n");
		} else {
			out.push_str(",\n");
		}
	}

	Ok(())
}
