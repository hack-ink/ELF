use std::{cmp::Ordering, collections::HashSet};

use uuid::Uuid;

use elf_service::SearchIndexItem;

use super::types::{EvalSummary, ExpectedKind, MergedQuery, Metrics, QueryReport};

pub(super) fn retrieval_top_rank_retention(
	candidates: &[elf_service::search::TraceReplayCandidate],
	note_ids: &[Uuid],
	max_retrieval_rank: u32,
) -> (usize, usize, f64) {
	let mut top_notes = HashSet::new();

	for candidate in candidates {
		if candidate.retrieval_rank == 0 || candidate.retrieval_rank > max_retrieval_rank {
			continue;
		}

		top_notes.insert(candidate.note_id);
	}

	let total = top_notes.len();

	if total == 0 {
		return (0, 0, 0.0);
	}

	let out_set: HashSet<Uuid> = note_ids.iter().copied().collect();
	let retained = top_notes.intersection(&out_set).count();
	let retention = retained as f64 / total as f64;

	(total, retained, retention)
}

pub(super) fn churn_against_baseline_at_k(
	baseline: &[Uuid],
	other: &[Uuid],
	k: usize,
) -> (f64, f64) {
	let k = k.max(1);
	let mut positional_diff = 0_usize;

	for idx in 0..k {
		let a = baseline.get(idx);
		let b = other.get(idx);

		if a != b {
			positional_diff += 1;
		}
	}

	let positional_churn = positional_diff as f64 / k as f64;
	let base_set: HashSet<Uuid> = baseline.iter().take(k).copied().collect();
	let other_set: HashSet<Uuid> = other.iter().take(k).copied().collect();
	let overlap = base_set.intersection(&other_set).count();
	let set_churn = 1.0 - (overlap as f64 / k as f64);

	(positional_churn, set_churn)
}

pub(super) fn unique_items(items: &[SearchIndexItem]) -> Vec<SearchIndexItem> {
	let mut seen = HashSet::new();
	let mut out = Vec::new();

	for item in items {
		if seen.insert(item.note_id) {
			out.push(item.clone());
		}
	}

	out
}

pub(super) fn compute_metrics(retrieved: &[Uuid], expected: &HashSet<Uuid>) -> Metrics {
	let expected_count = expected.len();
	let mut relevant_count = 0_usize;
	let mut dcg = 0.0_f64;
	let mut rr = 0.0_f64;
	let mut first_hit: Option<usize> = None;

	for (idx, id) in retrieved.iter().enumerate() {
		if expected.contains(id) {
			relevant_count += 1;

			let rank = idx + 1;
			let denom = (rank as f64 + 1.0).log2();

			dcg += 1.0 / denom;

			if first_hit.is_none() {
				first_hit = Some(rank);
			}
		}
	}

	if let Some(rank) = first_hit {
		rr = 1.0 / rank as f64;
	}

	let ideal_hits = expected_count.min(retrieved.len());
	let mut idcg = 0.0_f64;

	for idx in 0..ideal_hits {
		let rank = idx + 1;
		let denom = (rank as f64 + 1.0).log2();

		idcg += 1.0 / denom;
	}

	let ndcg = if idcg > 0.0 { dcg / idcg } else { 0.0 };
	let precision_at_k =
		if retrieved.is_empty() { 0.0 } else { relevant_count as f64 / retrieved.len() as f64 };
	let recall_at_k =
		if expected_count == 0 { 0.0 } else { relevant_count as f64 / expected_count as f64 };

	Metrics { recall_at_k, precision_at_k, rr, ndcg, relevant_count }
}

pub(super) fn compute_metrics_for_keys(
	retrieved: &[Option<String>],
	expected: &HashSet<String>,
) -> Metrics {
	let expected_count = expected.len();
	let mut matched: HashSet<String> = HashSet::new();
	let mut relevant_count = 0_usize;
	let mut dcg = 0.0_f64;
	let mut rr = 0.0_f64;
	let mut first_hit: Option<usize> = None;

	for (idx, maybe_key) in retrieved.iter().enumerate() {
		let Some(key) = maybe_key else {
			continue;
		};

		if expected.contains(key) && !matched.contains(key) {
			matched.insert(key.clone());

			relevant_count += 1;

			let rank = idx + 1;
			let denom = (rank as f64 + 1.0).log2();

			dcg += 1.0 / denom;

			if first_hit.is_none() {
				first_hit = Some(rank);
			}
		}
	}

	if let Some(rank) = first_hit {
		rr = 1.0 / rank as f64;
	}

	let ideal_hits = expected_count.min(retrieved.len());
	let mut idcg = 0.0_f64;

	for idx in 0..ideal_hits {
		let rank = idx + 1;
		let denom = (rank as f64 + 1.0).log2();

		idcg += 1.0 / denom;
	}

	let ndcg = if idcg > 0.0 { dcg / idcg } else { 0.0 };
	let precision_at_k =
		if retrieved.is_empty() { 0.0 } else { relevant_count as f64 / retrieved.len() as f64 };
	let recall_at_k =
		if expected_count == 0 { 0.0 } else { relevant_count as f64 / expected_count as f64 };

	Metrics { recall_at_k, precision_at_k, rr, ndcg, relevant_count }
}

pub(super) fn compute_metrics_for_query(
	merged: &MergedQuery,
	retrieved_note_ids: &[Uuid],
	retrieved_keys: &[Option<String>],
) -> (Metrics, usize) {
	match merged.expected_kind {
		ExpectedKind::NoteId => {
			let expected: HashSet<Uuid> = merged.expected_note_ids.iter().copied().collect();
			let expected_count = expected.len();

			(compute_metrics(retrieved_note_ids, &expected), expected_count)
		},
		ExpectedKind::Key => {
			let expected: HashSet<String> = merged.expected_keys.iter().cloned().collect();
			let expected_count = expected.len();

			(compute_metrics_for_keys(retrieved_keys, &expected), expected_count)
		},
	}
}

pub(super) fn summarize(reports: &[QueryReport], latencies_ms: &[f64]) -> EvalSummary {
	let count = reports.len().max(1) as f64;
	let avg_recall_at_k = reports.iter().map(|r| r.recall_at_k).sum::<f64>() / count;
	let avg_precision_at_k = reports.iter().map(|r| r.precision_at_k).sum::<f64>() / count;
	let mean_rr = reports.iter().map(|r| r.rr).sum::<f64>() / count;
	let mean_ndcg = reports.iter().map(|r| r.ndcg).sum::<f64>() / count;
	let avg_retrieved_summary_chars =
		reports.iter().map(|r| r.retrieved_summary_chars as f64).sum::<f64>() / count;
	let mut sorted = latencies_ms.to_vec();

	sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

	let p50 = percentile(&sorted, 0.50);
	let p95 = percentile(&sorted, 0.95);

	EvalSummary {
		avg_recall_at_k,
		avg_precision_at_k,
		mean_rr,
		mean_ndcg,
		latency_ms_p50: p50,
		latency_ms_p95: p95,
		avg_retrieved_summary_chars,
		stability: None,
	}
}

fn percentile(values: &[f64], percentile: f64) -> f64 {
	if values.is_empty() {
		return 0.0;
	}

	let clamped = percentile.clamp(0.0, 1.0);
	let pos = clamped * (values.len() as f64 - 1.0);
	let lower = pos.floor() as usize;
	let upper = pos.ceil() as usize;

	if lower == upper {
		values[lower]
	} else {
		let weight = pos - lower as f64;

		values[lower] * (1.0 - weight) + values[upper] * weight
	}
}
