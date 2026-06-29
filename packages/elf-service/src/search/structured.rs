use super::*;

pub(super) fn build_structured_field_matches(
	rows: Vec<FieldHit>,
) -> (Vec<Uuid>, HashMap<Uuid, Vec<String>>) {
	let mut structured_matches: HashMap<Uuid, HashSet<String>> = HashMap::new();
	let mut ordered_note_ids = Vec::new();
	let mut seen_notes = HashSet::new();

	for row in rows {
		let label = match row.field_kind.as_str() {
			"summary" => "summary",
			"fact" => "facts",
			"concept" => "concepts",
			_ => continue,
		};

		structured_matches.entry(row.note_id).or_default().insert(label.to_string());

		if seen_notes.insert(row.note_id) {
			ordered_note_ids.push(row.note_id);
		}
	}

	let mut structured_matches_out: HashMap<Uuid, Vec<String>> = HashMap::new();

	for (note_id, fields) in structured_matches {
		let mut fields: Vec<String> = fields.into_iter().collect();

		fields.sort();
		structured_matches_out.insert(note_id, fields);
	}

	(ordered_note_ids, structured_matches_out)
}

pub(super) fn build_structured_field_candidates(
	candidate_k: u32,
	ordered_note_ids: Vec<Uuid>,
	best_by_note: HashMap<Uuid, (Uuid, i32)>,
	embed_version: &str,
) -> Vec<ChunkCandidate> {
	let mut structured_candidates = Vec::new();
	let mut next_rank = 1_u32;

	for note_id in ordered_note_ids {
		if structured_candidates.len() >= candidate_k as usize {
			break;
		}

		let Some((chunk_id, chunk_index)) = best_by_note.get(&note_id) else { continue };

		structured_candidates.push(ChunkCandidate {
			chunk_id: *chunk_id,
			note_id,
			chunk_index: *chunk_index,
			retrieval_rank: next_rank,
			retrieval_score: None,
			scope: None,
			updated_at: None,
			embedding_version: Some(embed_version.to_string()),
		});

		next_rank = next_rank.saturating_add(1);
	}

	structured_candidates
}

pub(super) fn build_deterministic_query_tokens(cfg: &Config, query: &str) -> Vec<String> {
	if cfg.ranking.deterministic.enabled
		&& cfg.ranking.deterministic.lexical.enabled
		&& cfg.ranking.deterministic.lexical.max_query_terms > 0
	{
		ranking::tokenize_query(query, cfg.ranking.deterministic.lexical.max_query_terms as usize)
	} else {
		Vec::new()
	}
}
