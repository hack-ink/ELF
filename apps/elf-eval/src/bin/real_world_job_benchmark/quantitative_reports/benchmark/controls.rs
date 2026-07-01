use crate::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct QuantitativeBenchmarkControls {
	pub(crate) same_corpus_required: bool,
	pub(crate) same_task_required: bool,
	pub(crate) ranked_candidates_required_for_ranking_metrics: bool,
	pub(crate) explicit_relevance_judgments_required_for_leaderboard: bool,
	pub(crate) minimum_query_count_for_leaderboard: usize,
	pub(crate) current_query_count: usize,
	pub(crate) current_ranking_query_count: usize,
	pub(crate) current_explicit_qrel_query_count: usize,
	pub(crate) leaderboard_claim_allowed: bool,
	pub(crate) leakage_control: String,
}
