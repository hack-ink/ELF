mod cache;
mod diversity;
mod policy;
mod query;
mod retrieval;
mod text;

pub(super) use cache::{
	build_cached_scores, build_expansion_cache_key, build_rerank_cache_key, cache_key_prefix,
	decode_json, hash_query,
};
pub(super) use diversity::{
	attach_diversity_decisions_to_trace_candidates, build_diversity_explain, build_rerank_ranks,
	build_rerank_ranks_for_replay, extract_replay_diversity_decisions, select_diverse_results,
};
pub(super) use policy::{
	NormalizationKind, build_config_snapshot, build_policy_snapshot, hash_policy_snapshot,
	resolve_blend_policy, resolve_diversity_policy, resolve_retrieval_sources_policy,
	resolve_scopes, retrieval_weight_for_rank,
};
pub(super) use query::{
	build_expansion_messages, expansion_mode_label, normalize_queries, resolve_expansion_mode,
	should_expand_dynamic,
};
pub(super) use retrieval::{
	candidate_matches_note, cmp_f32_desc, collect_chunk_candidates, collect_neighbor_pairs,
	merge_retrieval_candidates, rank_normalize, stitch_snippet,
};
pub(super) use text::{
	build_dense_embedding_input, build_scope_context_boost_by_scope,
	compute_deterministic_ranking_terms, match_terms_in_text, merge_matched_fields, tokenize_query,
};

#[cfg(test)]
pub(super) use policy::{BlendSegment, ResolvedDiversityPolicy, ResolvedRetrievalSourcesPolicy};
#[cfg(test)] pub(super) use text::{lexical_overlap_ratio, scope_description_boost};
