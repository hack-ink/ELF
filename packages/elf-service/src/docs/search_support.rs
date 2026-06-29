use super::*;

pub(super) fn docs_search_l0_deduplicated_chunks(
	scored: &[ScoredPoint],
	candidate_k: usize,
) -> Result<Vec<(Uuid, f32)>> {
	let mut seen = HashSet::new();
	let mut chunks = Vec::new();

	for point in scored.iter().take(candidate_k) {
		let chunk_id = parse_scored_point_uuid_id(point)?;

		if seen.insert(chunk_id) {
			chunks.push((chunk_id, point.score));
		}
	}

	Ok(chunks)
}

pub(super) fn docs_search_l0_project_items(
	scored_chunks: &[(Uuid, f32)],
	rows: &HashMap<Uuid, DocSearchRow>,
	caller_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<SharedSpaceGrantKey>,
) -> Vec<DocsSearchL0Item> {
	let mut items = Vec::with_capacity(scored_chunks.len());

	for (chunk_id, score) in scored_chunks {
		let Some(row) = rows.get(chunk_id) else { continue };

		if !doc_read_allowed(
			caller_agent_id,
			allowed_scopes,
			shared_grants,
			row.agent_id.as_str(),
			row.scope.as_str(),
		) {
			continue;
		}

		items.push(DocsSearchL0Item {
			doc_id: row.doc_id,
			chunk_id: *chunk_id,
			pointer: build_docs_l0_pointer(row, *chunk_id),
			score: *score,
			snippet: truncate_bytes(row.chunk_text.as_str(), DEFAULT_L0_MAX_BYTES),
			scope: row.scope.clone(),
			doc_type: row.doc_type.clone(),
			project_id: row.project_id.clone(),
			agent_id: row.agent_id.clone(),
			updated_at: row.updated_at,
			content_hash: row.content_hash.clone(),
			chunk_hash: row.chunk_hash.clone(),
		});
	}

	items
}

pub(super) fn apply_doc_recency_boost(
	items: &mut [DocsSearchL0Item],
	now: OffsetDateTime,
	recency_tau_days: f32,
	tie_breaker_weight: f32,
) {
	if tie_breaker_weight <= 0.0 || items.is_empty() {
		return;
	}

	for item in items.iter_mut() {
		let age_days = ((now - item.updated_at).as_seconds_f32() / 86_400.0).max(0.0);
		let recency_decay =
			if recency_tau_days > 0.0 { (-age_days / recency_tau_days).exp() } else { 1.0 };

		item.score += tie_breaker_weight * recency_decay;
	}
}

pub(super) fn record_result_projection_stage(
	trajectory: &mut DocTrajectoryBuilder,
	pre_authorization_candidates: usize,
	returned_items: usize,
	recency_tau_days: f32,
	tie_breaker_weight: f32,
) {
	trajectory.push(
		"result_projection",
		serde_json::json!({
			"pre_authorization_candidates": pre_authorization_candidates,
			"returned_items": returned_items,
			"recency_tau_days": recency_tau_days,
			"tie_breaker_weight": tie_breaker_weight,
			"recency_boost_applied": tie_breaker_weight > 0.0 && !pre_authorization_candidates.eq(&0),
		}),
	)
}
