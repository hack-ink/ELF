use elf_config::{
	Chunking, Config, EmbeddingProviderConfig, Lifecycle, LlmProviderConfig, Memory, MemoryPolicy,
	MemoryPolicyRule, Postgres, ProviderConfig, Providers, Qdrant, Ranking, RankingBlend,
	RankingBlendSegment, RankingDeterministic, RankingDeterministicDecay, RankingDeterministicHits,
	RankingDeterministicLexical, RankingDiversity, RankingRetrievalSources, ReadProfiles,
	ScopePrecedence, ScopeWriteAllowed, Scopes, Search, SearchCache, SearchDynamic,
	SearchExpansion, SearchExplain, SearchPrefilter, Security, Service, Storage, TtlDays,
};

use elf_domain::memory_policy::{
	MemoryPolicyDecision, MemoryPolicyEvaluation, evaluate_memory_policy,
};

fn memory_policy_config(policy: MemoryPolicy) -> Config {
	Config {
		service: Service {
			http_bind: "127.0.0.1:8080".to_string(),
			mcp_bind: "127.0.0.1:8082".to_string(),
			admin_bind: "127.0.0.1:8081".to_string(),
			log_level: "info".to_string(),
		},
		storage: Storage {
			postgres: Postgres {
				dsn: "postgres://user:pass@localhost/db".to_string(),
				pool_max_conns: 1,
			},
			qdrant: Qdrant {
				url: "http://localhost".to_string(),
				collection: "mem_notes_v2".to_string(),
				vector_dim: 4_096,
			},
		},
		providers: Providers {
			embedding: EmbeddingProviderConfig {
				provider_id: "p".to_string(),
				api_base: "http://localhost".to_string(),
				api_key: "key".to_string(),
				path: "/".to_string(),
				model: "m".to_string(),
				dimensions: 3,
				timeout_ms: 1_000,
				default_headers: serde_json::Map::new(),
			},
			rerank: ProviderConfig {
				provider_id: "p".to_string(),
				api_base: "http://localhost".to_string(),
				api_key: "key".to_string(),
				path: "/".to_string(),
				model: "m".to_string(),
				timeout_ms: 1_000,
				default_headers: serde_json::Map::new(),
			},
			llm_extractor: LlmProviderConfig {
				provider_id: "p".to_string(),
				api_base: "http://localhost".to_string(),
				api_key: "key".to_string(),
				path: "/".to_string(),
				model: "m".to_string(),
				temperature: 0.1,
				timeout_ms: 1_000,
				default_headers: serde_json::Map::new(),
			},
		},
		scopes: Scopes {
			allowed: vec!["agent_private".to_string()],
			read_profiles: ReadProfiles {
				private_only: vec!["agent_private".to_string()],
				private_plus_project: vec!["agent_private".to_string()],
				all_scopes: vec!["agent_private".to_string()],
			},
			precedence: ScopePrecedence { agent_private: 30, project_shared: 20, org_shared: 10 },
			write_allowed: ScopeWriteAllowed {
				agent_private: true,
				project_shared: true,
				org_shared: true,
			},
		},
		memory: Memory {
			max_notes_per_add_event: 3,
			max_note_chars: 240,
			dup_sim_threshold: 0.92,
			update_sim_threshold: 0.85,
			candidate_k: 60,
			top_k: 12,
			policy,
		},
		search: Search {
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
			recursive: Default::default(),
			graph_context: Default::default(),
		},
		ranking: Ranking {
			recency_tau_days: 60.0,
			tie_breaker_weight: 0.1,
			deterministic: RankingDeterministic {
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
			},
			blend: RankingBlend {
				enabled: true,
				rerank_normalization: "rank".to_string(),
				retrieval_normalization: "rank".to_string(),
				segments: vec![RankingBlendSegment {
					max_retrieval_rank: 10,
					retrieval_weight: 0.5,
				}],
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
		},
		lifecycle: Lifecycle {
			ttl_days: TtlDays {
				plan: 14,
				fact: 180,
				preference: 0,
				constraint: 0,
				decision: 0,
				profile: 0,
			},
			purge_deleted_after_days: 30,
			purge_deprecated_after_days: 180,
		},
		security: Security {
			bind_localhost_only: true,
			reject_cjk: true,
			redact_secrets_on_write: true,
			evidence_min_quotes: 1,
			evidence_max_quotes: 2,
			evidence_max_quote_chars: 320,
			auth_mode: "off".to_string(),
			auth_keys: vec![],
		},
		chunking: Chunking {
			enabled: true,
			max_tokens: 512,
			overlap_tokens: 128,
			tokenizer_repo: "REPLACE_ME".to_string(),
		},
		context: None,
		mcp: None,
	}
}

#[test]
fn selects_note_type_and_scope_rule_before_note_type() {
	let cfg = memory_policy_config(MemoryPolicy {
		rules: vec![
			MemoryPolicyRule {
				note_type: Some("fact".to_string()),
				scope: None,
				min_confidence: Some(0.2),
				min_importance: None,
			},
			MemoryPolicyRule {
				note_type: Some("fact".to_string()),
				scope: Some("agent_private".to_string()),
				min_confidence: Some(0.9),
				min_importance: None,
			},
			MemoryPolicyRule {
				note_type: None,
				scope: Some("agent_private".to_string()),
				min_confidence: Some(0.0),
				min_importance: None,
			},
		],
	});

	let MemoryPolicyEvaluation { decision, matched_rule } = evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.5,
		0.5,
		MemoryPolicyDecision::Remember,
	);

	assert_eq!(decision, MemoryPolicyDecision::Ignore);
	assert!(matched_rule.is_some());
	assert_eq!(matched_rule.unwrap().note_type.as_deref(), Some("fact"));
	assert_eq!(matched_rule.unwrap().scope.as_deref(), Some("agent_private"));
	assert_eq!(matched_rule.unwrap().min_confidence, Some(0.9));
}

#[test]
fn downgrades_only_remember_or_update() {
	let cfg = memory_policy_config(MemoryPolicy {
		rules: vec![MemoryPolicyRule {
			note_type: Some("fact".to_string()),
			scope: Some("agent_private".to_string()),
			min_confidence: Some(0.9),
			min_importance: None,
		}],
	});

	let remember = evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.5,
		0.5,
		MemoryPolicyDecision::Remember,
	);
	assert_eq!(remember.decision, MemoryPolicyDecision::Ignore);

	let update = evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.5,
		0.5,
		MemoryPolicyDecision::Update,
	);
	assert_eq!(update.decision, MemoryPolicyDecision::Ignore);

	let ignored = evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.5,
		0.5,
		MemoryPolicyDecision::Ignore,
	);
	assert_eq!(ignored.decision, MemoryPolicyDecision::Ignore);

	let rejected = evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.5,
		0.5,
		MemoryPolicyDecision::Reject,
	);
	assert_eq!(rejected.decision, MemoryPolicyDecision::Reject);
}

#[test]
fn note_type_only_beats_scope_only() {
	let cfg = memory_policy_config(MemoryPolicy {
		rules: vec![
			MemoryPolicyRule {
				note_type: None,
				scope: Some("agent_private".to_string()),
				min_confidence: Some(0.1),
				min_importance: None,
			},
			MemoryPolicyRule {
				note_type: Some("fact".to_string()),
				scope: None,
				min_confidence: Some(0.1),
				min_importance: None,
			},
		],
	});

	let output = evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.2,
		0.0,
		MemoryPolicyDecision::Remember,
	);

	assert_eq!(output.decision, MemoryPolicyDecision::Remember);
	assert_eq!(output.matched_rule.and_then(|rule| rule.note_type.as_deref()), Some("fact"));
	assert_eq!(output.matched_rule.and_then(|rule| rule.scope.as_deref()), None);
}

#[test]
fn scope_only_beats_fallback_none() {
	let cfg = memory_policy_config(MemoryPolicy {
		rules: vec![
			MemoryPolicyRule {
				note_type: None,
				scope: None,
				min_confidence: Some(0.1),
				min_importance: None,
			},
			MemoryPolicyRule {
				note_type: None,
				scope: Some("agent_private".to_string()),
				min_confidence: Some(0.1),
				min_importance: None,
			},
		],
	});

	let output = evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.2,
		0.0,
		MemoryPolicyDecision::Remember,
	);

	assert_eq!(output.decision, MemoryPolicyDecision::Remember);
	assert_eq!(output.matched_rule.and_then(|rule| rule.note_type.as_deref()), None);
	assert_eq!(output.matched_rule.and_then(|rule| rule.scope.as_deref()), Some("agent_private"));
}

#[test]
fn confidence_meets_minimum_is_not_a_downgrade() {
	let cfg = memory_policy_config(MemoryPolicy {
		rules: vec![MemoryPolicyRule {
			note_type: Some("fact".to_string()),
			scope: Some("agent_private".to_string()),
			min_confidence: Some(0.5),
			min_importance: None,
		}],
	});

	let output = evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.5,
		0.0,
		MemoryPolicyDecision::Remember,
	);

	assert_eq!(output.decision, MemoryPolicyDecision::Remember);
}

#[test]
fn importance_meets_minimum_is_not_a_downgrade() {
	let cfg = memory_policy_config(MemoryPolicy {
		rules: vec![MemoryPolicyRule {
			note_type: Some("fact".to_string()),
			scope: Some("agent_private".to_string()),
			min_confidence: None,
			min_importance: Some(0.7),
		}],
	});

	let output = evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.0,
		0.7,
		MemoryPolicyDecision::Remember,
	);

	assert_eq!(output.decision, MemoryPolicyDecision::Remember);
}

#[test]
fn non_finite_metrics_fail_threshold() {
	let cfg = memory_policy_config(MemoryPolicy {
		rules: vec![MemoryPolicyRule {
			note_type: Some("fact".to_string()),
			scope: Some("agent_private".to_string()),
			min_confidence: Some(0.9),
			min_importance: None,
		}],
	});

	let output = evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		f64::NAN,
		0.5,
		MemoryPolicyDecision::Remember,
	);
	assert_eq!(output.decision, MemoryPolicyDecision::Ignore);
}

#[test]
fn missing_threshold_does_not_change_decision() {
	let cfg = memory_policy_config(MemoryPolicy {
		rules: vec![MemoryPolicyRule {
			note_type: Some("fact".to_string()),
			scope: Some("agent_private".to_string()),
			min_confidence: None,
			min_importance: None,
		}],
	});

	let output = evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.0,
		0.0,
		MemoryPolicyDecision::Remember,
	);
	assert_eq!(output.decision, MemoryPolicyDecision::Remember);
}
