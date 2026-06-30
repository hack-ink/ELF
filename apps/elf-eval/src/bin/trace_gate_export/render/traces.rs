use color_eyre::Result;

use crate::{rows::TraceRow, sql};

pub(super) fn render_traces(out: &mut String, traces: &[TraceRow]) -> Result<()> {
	if traces.is_empty() {
		return Ok(());
	}

	out.push_str("INSERT INTO search_traces (\n");
	out.push_str("	trace_id,\n");
	out.push_str("	tenant_id,\n");
	out.push_str("	project_id,\n");
	out.push_str("	agent_id,\n");
	out.push_str("	read_profile,\n");
	out.push_str("	query,\n");
	out.push_str("	expansion_mode,\n");
	out.push_str("	expanded_queries,\n");
	out.push_str("	allowed_scopes,\n");
	out.push_str("	candidate_count,\n");
	out.push_str("	top_k,\n");
	out.push_str("	config_snapshot,\n");
	out.push_str("	trace_version,\n");
	out.push_str("	created_at,\n");
	out.push_str("	expires_at\n");
	out.push_str(")\nVALUES\n");

	for (idx, row) in traces.iter().enumerate() {
		out.push_str("	(");
		out.push_str(&sql::sql_uuid(&row.trace_id));
		out.push_str(", ");
		out.push_str(&sql::sql_text(&row.tenant_id));
		out.push_str(", ");
		out.push_str(&sql::sql_text(&row.project_id));
		out.push_str(", ");
		out.push_str(&sql::sql_text(&row.agent_id));
		out.push_str(", ");
		out.push_str(&sql::sql_text(&row.read_profile));
		out.push_str(", ");
		out.push_str(&sql::sql_text(&row.query));
		out.push_str(", ");
		out.push_str(&sql::sql_text(&row.expansion_mode));
		out.push_str(", ");
		out.push_str(&sql::sql_jsonb(&row.expanded_queries)?);
		out.push_str(", ");
		out.push_str(&sql::sql_jsonb(&row.allowed_scopes)?);
		out.push_str(", ");
		out.push_str(&row.candidate_count.to_string());
		out.push_str(", ");
		out.push_str(&row.top_k.to_string());
		out.push_str(", ");
		out.push_str(&sql::sql_jsonb(&row.config_snapshot)?);
		out.push_str(", ");
		out.push_str(&row.trace_version.to_string());
		out.push_str(", ");
		out.push_str(&sql::sql_timestamptz(&row.created_at)?);
		out.push_str(", ");
		out.push_str(&sql::sql_timestamptz(&row.expires_at)?);
		out.push(')');

		if idx + 1 == traces.len() {
			out.push_str(";\n\n");
		} else {
			out.push_str(",\n");
		}
	}

	Ok(())
}
