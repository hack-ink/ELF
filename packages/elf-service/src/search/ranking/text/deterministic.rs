use time::OffsetDateTime;

use crate::search::{DeterministicRankingTerms, ranking::text::tokenization};
use elf_config::Config;

pub(crate) fn compute_deterministic_ranking_terms(
	cfg: &Config,
	query_tokens: &[String],
	snippet: &str,
	note_hit_count: i64,
	note_last_hit_at: Option<OffsetDateTime>,
	age_days: f32,
	now: OffsetDateTime,
) -> DeterministicRankingTerms {
	let det = &cfg.ranking.deterministic;

	if !det.enabled {
		return DeterministicRankingTerms::default();
	}

	let mut out = DeterministicRankingTerms::default();

	if det.lexical.enabled && det.lexical.weight > 0.0 && !query_tokens.is_empty() {
		let ratio = tokenization::lexical_overlap_ratio(
			query_tokens,
			snippet,
			det.lexical.max_text_terms as usize,
		);

		out.lexical_overlap_ratio = ratio;

		let min_ratio = det.lexical.min_ratio.clamp(0.0, 1.0);
		let scaled = if ratio >= min_ratio && min_ratio < 1.0 {
			((ratio - min_ratio) / (1.0 - min_ratio)).clamp(0.0, 1.0)
		} else if ratio >= 1.0 && min_ratio >= 1.0 {
			1.0
		} else {
			0.0
		};

		out.lexical_bonus = det.lexical.weight * scaled;
	}
	if det.hits.enabled && det.hits.weight > 0.0 {
		let hit_count = note_hit_count.max(0);

		out.hit_count = hit_count;

		let half = det.hits.half_saturation;
		let hit_saturation = if half > 0.0 && hit_count > 0 {
			let hc = hit_count as f32;

			(hc / (hc + half)).clamp(0.0, 1.0)
		} else {
			0.0
		};
		let last_hit_age_days =
			note_last_hit_at.map(|ts| ((now - ts).as_seconds_f32() / 86_400.0).max(0.0));

		out.last_hit_age_days = last_hit_age_days;

		let tau = det.hits.last_hit_tau_days;
		let recency = if tau > 0.0 {
			match last_hit_age_days {
				Some(days) => (-days / tau).exp(),
				None => 1.0,
			}
		} else {
			1.0
		};

		out.hit_boost = det.hits.weight * hit_saturation * recency;
	}
	if det.decay.enabled && det.decay.weight > 0.0 {
		let age_days = age_days.max(0.0);
		let tau = det.decay.tau_days;
		let staleness = if tau > 0.0 { 1.0 - (-age_days / tau).exp() } else { 0.0 };

		out.decay_penalty = -det.decay.weight * staleness.clamp(0.0, 1.0);
	}

	out
}
