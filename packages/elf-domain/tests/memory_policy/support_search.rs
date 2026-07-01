use elf_config::{
	Search, SearchCache, SearchDynamic, SearchExpansion, SearchExplain, SearchGraphContext,
	SearchPrefilter, SearchRecursive,
};

pub(crate) fn memory_policy_search_config() -> Search {
	Search {
		expansion: SearchExpansion {
			mode: "off".to_string(),
			max_queries: 4,
			include_original: true,
		},
		dynamic: SearchDynamic { min_candidates: 10, min_top_score: 0.12 },
		prefilter: SearchPrefilter { max_candidates: 0 },
		cache: SearchCache {
			enabled: true,
			expansion_ttl_days: 7,
			rerank_ttl_days: 7,
			max_payload_bytes: Some(262_144),
		},
		explain: SearchExplain {
			retention_days: 7,
			capture_candidates: false,
			candidate_retention_days: 2,
			write_mode: "outbox".to_string(),
		},
		recursive: SearchRecursive {
			enabled: false,
			max_depth: 2,
			max_children_per_node: 4,
			max_nodes_per_scope: 32,
			max_total_nodes: 256,
		},
		graph_context: SearchGraphContext {
			enabled: false,
			max_facts_per_item: 16,
			max_evidence_notes_per_fact: 16,
		},
	}
}
