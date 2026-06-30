use color_eyre::Result;

use crate::{rows::CandidateRow, sql};

pub(super) fn render_candidates(out: &mut String, candidates: &[CandidateRow]) -> Result<()> {
	if candidates.is_empty() {
		return Ok(());
	}

	out.push_str("INSERT INTO search_trace_candidates (\n");
	out.push_str("	candidate_id,\n");
	out.push_str("	trace_id,\n");
	out.push_str("	note_id,\n");
	out.push_str("	chunk_id,\n");
	out.push_str("	chunk_index,\n");
	out.push_str("	snippet,\n");
	out.push_str("	candidate_snapshot,\n");
	out.push_str("	retrieval_rank,\n");
	out.push_str("	rerank_score,\n");
	out.push_str("	note_scope,\n");
	out.push_str("	note_importance,\n");
	out.push_str("	note_updated_at,\n");
	out.push_str("	note_hit_count,\n");
	out.push_str("	note_last_hit_at,\n");
	out.push_str("	created_at,\n");
	out.push_str("	expires_at\n");
	out.push_str(")\nVALUES\n");

	for (idx, row) in candidates.iter().enumerate() {
		out.push_str("	(");
		out.push_str(&sql::sql_uuid(&row.candidate_id));
		out.push_str(", ");
		out.push_str(&sql::sql_uuid(&row.trace_id));
		out.push_str(", ");
		out.push_str(&sql::sql_uuid(&row.note_id));
		out.push_str(", ");
		out.push_str(&sql::sql_uuid(&row.chunk_id));
		out.push_str(", ");
		out.push_str(&row.chunk_index.to_string());
		out.push_str(", ");
		out.push_str(&sql::sql_text(&row.snippet));
		out.push_str(", ");
		out.push_str(&sql::sql_jsonb(&row.candidate_snapshot)?);
		out.push_str(", ");
		out.push_str(&row.retrieval_rank.to_string());
		out.push_str(", ");
		out.push_str(&sql::sql_f32(row.rerank_score));
		out.push_str(", ");
		out.push_str(&sql::sql_text(&row.note_scope));
		out.push_str(", ");
		out.push_str(&sql::sql_f32(row.note_importance));
		out.push_str(", ");
		out.push_str(&sql::sql_timestamptz(&row.note_updated_at)?);
		out.push_str(", ");
		out.push_str(&row.note_hit_count.to_string());
		out.push_str(", ");
		out.push_str(&sql::sql_opt_timestamptz(&row.note_last_hit_at)?);
		out.push_str(", ");
		out.push_str(&sql::sql_timestamptz(&row.created_at)?);
		out.push_str(", ");
		out.push_str(&sql::sql_timestamptz(&row.expires_at)?);
		out.push(')');

		if idx + 1 == candidates.len() {
			out.push_str(";\n\n");
		} else {
			out.push_str(",\n");
		}
	}

	Ok(())
}
