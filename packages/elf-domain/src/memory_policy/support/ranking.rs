use elf_config::{
	Ranking, RankingBlend, RankingBlendSegment, RankingDeterministic, RankingDeterministicDecay,
	RankingDeterministicHits, RankingDeterministicLexical, RankingDiversity,
	RankingRetrievalSources,
};

pub(crate) fn test_ranking_config() -> Ranking {
	Ranking {
		recency_tau_days: 60.0,
		tie_breaker_weight: 0.1,
		deterministic: test_ranking_deterministic_config(),
		blend: RankingBlend {
			enabled: true,
			rerank_normalization: "rank".to_string(),
			retrieval_normalization: "rank".to_string(),
			segments: vec![
				RankingBlendSegment { max_retrieval_rank: 3, retrieval_weight: 0.8 },
				RankingBlendSegment { max_retrieval_rank: 10, retrieval_weight: 0.5 },
				RankingBlendSegment { max_retrieval_rank: 1_000_000, retrieval_weight: 0.2 },
			],
		},
		diversity: RankingDiversity {
			enabled: true,
			sim_threshold: 0.88,
			mmr_lambda: 0.7,
			max_skips: 64,
		},
		retrieval_sources: RankingRetrievalSources {
			fusion_weight: 1.0,
			structured_field_weight: 1.0,
			fusion_priority: 1,
			structured_field_priority: 0,
		},
	}
}

fn test_ranking_deterministic_config() -> RankingDeterministic {
	RankingDeterministic {
		enabled: false,
		lexical: RankingDeterministicLexical {
			enabled: false,
			weight: 0.05,
			min_ratio: 0.3,
			max_query_terms: 16,
			max_text_terms: 1_024,
		},
		hits: RankingDeterministicHits {
			enabled: false,
			weight: 0.05,
			half_saturation: 8.0,
			last_hit_tau_days: 14.0,
		},
		decay: RankingDeterministicDecay { enabled: false, weight: 0.05, tau_days: 30.0 },
	}
}
