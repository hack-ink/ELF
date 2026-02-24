use serde::{Deserialize, Serialize};

use elf_config::{Config, MemoryPolicyRule};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryPolicyDecision {
	Remember,
	Update,
	Ignore,
	Reject,
}

#[derive(Debug)]
pub struct MemoryPolicyEvaluation<'a> {
	pub decision: MemoryPolicyDecision,
	pub matched_rule: Option<&'a MemoryPolicyRule>,
}

pub fn evaluate_memory_policy<'a>(
	cfg: &'a Config,
	note_type: &str,
	scope: &str,
	confidence: f64,
	importance: f64,
	base_decision: MemoryPolicyDecision,
) -> MemoryPolicyEvaluation<'a> {
	let matched_rule = select_memory_policy_rule(cfg, note_type, scope);
	let decision =
		if matches!(base_decision, MemoryPolicyDecision::Remember | MemoryPolicyDecision::Update)
			&& should_downgrade(matched_rule, confidence, importance)
		{
			MemoryPolicyDecision::Ignore
		} else {
			base_decision
		};

	MemoryPolicyEvaluation { decision, matched_rule }
}

fn select_memory_policy_rule<'a>(
	cfg: &'a Config,
	note_type: &str,
	scope: &str,
) -> Option<&'a MemoryPolicyRule> {
	let exact_match =
		cfg.memory.policy.rules.iter().find(|rule| matches_exact(note_type, scope, rule));

	if exact_match.is_some() {
		return exact_match;
	}

	let note_type_match =
		cfg.memory.policy.rules.iter().find(|rule| matches_note_type(note_type, rule));

	if note_type_match.is_some() {
		return note_type_match;
	}

	let scope_match = cfg.memory.policy.rules.iter().find(|rule| matches_scope(scope, rule));

	if scope_match.is_some() {
		return scope_match;
	}

	cfg.memory.policy.rules.iter().find(|rule| rule.note_type.is_none() && rule.scope.is_none())
}

fn matches_exact(note_type: &str, scope: &str, rule: &MemoryPolicyRule) -> bool {
	match (rule.note_type.as_deref(), rule.scope.as_deref()) {
		(Some(rule_type), Some(rule_scope)) => rule_type == note_type && rule_scope == scope,
		_ => false,
	}
}

fn matches_note_type(note_type: &str, rule: &MemoryPolicyRule) -> bool {
	match (rule.note_type.as_deref(), rule.scope.as_deref()) {
		(Some(rule_type), None) => rule_type == note_type,
		_ => false,
	}
}

fn matches_scope(scope: &str, rule: &MemoryPolicyRule) -> bool {
	match (rule.note_type.as_deref(), rule.scope.as_deref()) {
		(None, Some(rule_scope)) => rule_scope == scope,
		_ => false,
	}
}

fn should_downgrade(
	matched_rule: Option<&MemoryPolicyRule>,
	confidence: f64,
	importance: f64,
) -> bool {
	let Some(rule) = matched_rule else {
		return false;
	};

	if let Some(min_confidence) = rule.min_confidence
		&& (!confidence.is_finite() || confidence < f64::from(min_confidence))
	{
		return true;
	}
	if let Some(min_importance) = rule.min_importance
		&& (!importance.is_finite() || importance < f64::from(min_importance))
	{
		return true;
	}

	false
}

#[cfg(test)]
mod tests {
	use crate::memory_policy::{
		MemoryPolicyDecision, MemoryPolicyEvaluation, evaluate_memory_policy,
	};
	use elf_config::{
		Chunking, Config, EmbeddingProviderConfig, Lifecycle, LlmProviderConfig, Memory,
		MemoryPolicy, MemoryPolicyRule, Postgres, ProviderConfig, Providers, Qdrant, Ranking,
		RankingBlend, RankingBlendSegment, RankingDeterministic, RankingDeterministicDecay,
		RankingDeterministicHits, RankingDeterministicLexical, RankingDiversity,
		RankingRetrievalSources, ReadProfiles, ScopePrecedence, ScopeWriteAllowed, Scopes, Search,
		SearchCache, SearchDynamic, SearchExpansion, SearchExplain, SearchPrefilter, Security,
		Service, Storage, TtlDays,
	};

	fn test_config(policy: MemoryPolicy) -> Config {
		let mut cfg = test_default_config();

		cfg.memory.policy = policy;

		cfg
	}

	fn test_default_config() -> Config {
		Config {
			service: test_service_config(),
			storage: test_storage_config(),
			providers: test_providers_config(),
			scopes: test_scopes_config(),
			memory: test_memory_config(),
			search: test_search_config(),
			ranking: test_ranking_config(),
			lifecycle: test_lifecycle_config(),
			security: test_security_config(),
			chunking: test_chunking_config(),
			context: None,
			mcp: None,
		}
	}

	fn test_service_config() -> Service {
		Service {
			http_bind: "127.0.0.1:8080".to_string(),
			mcp_bind: "127.0.0.1:8082".to_string(),
			admin_bind: "127.0.0.1:8081".to_string(),
			log_level: "info".to_string(),
		}
	}

	fn test_storage_config() -> Storage {
		Storage {
			postgres: Postgres {
				dsn: "postgres://user:pass@localhost/db".to_string(),
				pool_max_conns: 1,
			},
			qdrant: Qdrant {
				url: "http://localhost".to_string(),
				collection: "mem_notes_v2".to_string(),
				docs_collection: "doc_chunks_v1".to_string(),
				vector_dim: 4_096,
			},
		}
	}

	fn test_providers_config() -> Providers {
		Providers {
			embedding: test_embedding_provider_config(),
			rerank: test_rerank_provider_config(),
			llm_extractor: test_llm_extractor_provider_config(),
		}
	}

	fn test_embedding_provider_config() -> EmbeddingProviderConfig {
		EmbeddingProviderConfig {
			provider_id: "p".to_string(),
			api_base: "http://localhost".to_string(),
			api_key: "key".to_string(),
			path: "/".to_string(),
			model: "m".to_string(),
			dimensions: 3,
			timeout_ms: 1_000,
			default_headers: Default::default(),
		}
	}

	fn test_rerank_provider_config() -> ProviderConfig {
		ProviderConfig {
			provider_id: "p".to_string(),
			api_base: "http://localhost".to_string(),
			api_key: "key".to_string(),
			path: "/".to_string(),
			model: "m".to_string(),
			timeout_ms: 1_000,
			default_headers: Default::default(),
		}
	}

	fn test_llm_extractor_provider_config() -> LlmProviderConfig {
		LlmProviderConfig {
			provider_id: "p".to_string(),
			api_base: "http://localhost".to_string(),
			api_key: "key".to_string(),
			path: "/".to_string(),
			model: "m".to_string(),
			temperature: 0.1,
			timeout_ms: 1_000,
			default_headers: Default::default(),
		}
	}

	fn test_scopes_config() -> Scopes {
		Scopes {
			allowed: vec!["agent_private".to_string()],
			read_profiles: test_read_profiles_config(),
			precedence: ScopePrecedence { agent_private: 30, project_shared: 20, org_shared: 10 },
			write_allowed: ScopeWriteAllowed {
				agent_private: true,
				project_shared: true,
				org_shared: true,
			},
		}
	}

	fn test_read_profiles_config() -> ReadProfiles {
		ReadProfiles {
			private_only: vec!["agent_private".to_string()],
			private_plus_project: vec!["agent_private".to_string()],
			all_scopes: vec!["agent_private".to_string()],
		}
	}

	fn test_memory_config() -> Memory {
		Memory {
			max_notes_per_add_event: 3,
			max_note_chars: 240,
			dup_sim_threshold: 0.92,
			update_sim_threshold: 0.85,
			candidate_k: 60,
			top_k: 12,
			policy: MemoryPolicy {
				rules: vec![
					MemoryPolicyRule {
						note_type: Some("fact".to_string()),
						scope: Some("agent_private".to_string()),
						min_confidence: Some(0.9),
						min_importance: Some(0.1),
					},
					MemoryPolicyRule {
						note_type: Some("preference".to_string()),
						scope: Some("agent_private".to_string()),
						min_confidence: Some(0.75),
						min_importance: None,
					},
					MemoryPolicyRule {
						note_type: Some("preference".to_string()),
						scope: None,
						min_confidence: Some(0.6),
						min_importance: None,
					},
					MemoryPolicyRule {
						note_type: None,
						scope: None,
						min_confidence: None,
						min_importance: None,
					},
				],
			},
		}
	}

	fn test_search_config() -> Search {
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
			recursive: Default::default(),
			graph_context: Default::default(),
		}
	}

	fn test_ranking_config() -> Ranking {
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

	fn test_lifecycle_config() -> Lifecycle {
		Lifecycle {
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
		}
	}

	fn test_security_config() -> Security {
		Security {
			bind_localhost_only: true,
			reject_cjk: true,
			redact_secrets_on_write: true,
			evidence_min_quotes: 1,
			evidence_max_quotes: 2,
			evidence_max_quote_chars: 320,
			auth_mode: "off".to_string(),
			auth_keys: vec![],
		}
	}

	fn test_chunking_config() -> Chunking {
		Chunking {
			enabled: true,
			max_tokens: 512,
			overlap_tokens: 128,
			tokenizer_repo: "REPLACE_ME".to_string(),
		}
	}
	#[test]
	fn policy_precedence_prefers_note_type_and_scope_over_note_type_only() {
		let cfg = test_config(MemoryPolicy {
			rules: vec![
				MemoryPolicyRule {
					note_type: Some("fact".to_string()),
					scope: None,
					min_confidence: Some(0.05),
					min_importance: None,
				},
				MemoryPolicyRule {
					note_type: Some("fact".to_string()),
					scope: Some("agent_private".to_string()),
					min_confidence: Some(0.95),
					min_importance: None,
				},
				MemoryPolicyRule {
					note_type: None,
					scope: Some("agent_private".to_string()),
					min_confidence: Some(0.40),
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

		let rule = matched_rule.expect("expected policy match");

		assert_eq!(rule.note_type.as_deref(), Some("fact"));
		assert_eq!(rule.scope.as_deref(), Some("agent_private"));
		assert_eq!(rule.min_confidence, Some(0.95));
		assert_eq!(rule.min_importance, None);
	}

	#[test]
	fn evaluate_downgrades_base_remember_update_only() {
		let cfg = test_config(MemoryPolicy {
			rules: vec![MemoryPolicyRule {
				note_type: Some("fact".to_string()),
				scope: Some("agent_private".to_string()),
				min_confidence: Some(0.9),
				min_importance: Some(0.5),
			}],
		});
		let remember = evaluate_memory_policy(
			&cfg,
			"fact",
			"agent_private",
			0.95,
			0.4,
			MemoryPolicyDecision::Remember,
		);

		assert_eq!(remember.decision, MemoryPolicyDecision::Ignore);

		let update = evaluate_memory_policy(
			&cfg,
			"fact",
			"agent_private",
			f64::NAN,
			f64::NAN,
			MemoryPolicyDecision::Update,
		);

		assert_eq!(update.decision, MemoryPolicyDecision::Ignore);

		let ignore = evaluate_memory_policy(
			&cfg,
			"fact",
			"agent_private",
			0.1,
			0.1,
			MemoryPolicyDecision::Ignore,
		);

		assert_eq!(ignore.decision, MemoryPolicyDecision::Ignore);

		let reject = evaluate_memory_policy(
			&cfg,
			"fact",
			"agent_private",
			0.1,
			0.1,
			MemoryPolicyDecision::Reject,
		);

		assert_eq!(reject.decision, MemoryPolicyDecision::Reject);
	}

	#[test]
	fn evaluate_without_matching_threshold_leaves_base_unchanged() {
		let cfg = test_config(MemoryPolicy {
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
}
