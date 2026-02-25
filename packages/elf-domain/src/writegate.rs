use regex::Regex;

use crate::english_gate;
use elf_config::Config;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectCode {
	RejectNonEnglish,
	RejectTooLong,
	RejectSecret,
	RejectInvalidType,
	RejectScopeDenied,
	RejectEmpty,
}

pub struct NoteInput {
	pub note_type: String,
	pub scope: String,
	pub text: String,
}

pub fn writegate(note: &NoteInput, cfg: &Config) -> Result<(), RejectCode> {
	if note.text.trim().is_empty() {
		return Err(RejectCode::RejectEmpty);
	}
	if !english_gate::is_english_natural_language(note.text.as_str()) {
		return Err(RejectCode::RejectNonEnglish);
	}
	if note.text.chars().count() as u32 > cfg.memory.max_note_chars {
		return Err(RejectCode::RejectTooLong);
	}
	if !is_allowed_type(&note.note_type) {
		return Err(RejectCode::RejectInvalidType);
	}
	if !cfg.scopes.allowed.iter().any(|scope| scope == &note.scope) {
		return Err(RejectCode::RejectScopeDenied);
	}
	if !scope_write_allowed(cfg, &note.scope) {
		return Err(RejectCode::RejectScopeDenied);
	}
	if contains_secrets(&note.text) {
		return Err(RejectCode::RejectSecret);
	}

	Ok(())
}

fn scope_write_allowed(cfg: &Config, scope: &str) -> bool {
	match scope {
		"agent_private" => cfg.scopes.write_allowed.agent_private,
		"project_shared" => cfg.scopes.write_allowed.project_shared,
		"org_shared" => cfg.scopes.write_allowed.org_shared,
		_ => false,
	}
}

fn is_allowed_type(note_type: &str) -> bool {
	matches!(note_type, "preference" | "constraint" | "decision" | "profile" | "fact" | "plan")
}

fn contains_secrets(text: &str) -> bool {
	let patterns = [
		r"(?i)-----BEGIN (RSA|OPENSSH|EC|DSA) PRIVATE KEY-----",
		r"(?i)ssh-rsa",
		r"(?i)sk-[a-z0-9]{20,}",
		r"(?i)api[_-]?key\s*[:=]\s*\S+",
		r"(?i)password\s*[:=]\s*\S+",
		r"(?i)secret\s*[:=]\s*\S+",
		r"(?i)token\s*[:=]\s*\S+",
		r"(?i)seed phrase",
	];

	for pattern in patterns {
		if Regex::new(pattern).map(|re| re.is_match(text)).unwrap_or(false) {
			return true;
		}
	}

	false
}

#[cfg(test)]
mod tests {
	use crate::writegate::{NoteInput, RejectCode, contains_secrets, writegate};
	use elf_config::{
		Chunking, Config, EmbeddingProviderConfig, Lifecycle, LlmProviderConfig, Memory,
		MemoryPolicy, Postgres, ProviderConfig, Providers, Qdrant, Ranking, RankingBlend,
		RankingBlendSegment, RankingDeterministic, RankingDeterministicDecay,
		RankingDeterministicHits, RankingDeterministicLexical, RankingDiversity,
		RankingRetrievalSources, ReadProfiles, ScopePrecedence, ScopeWriteAllowed, Scopes, Search,
		SearchCache, SearchDynamic, SearchExpansion, SearchExplain, SearchPrefilter, Security,
		Service, Storage, TtlDays,
	};

	fn test_ranking() -> Ranking {
		Ranking {
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

	fn config() -> Config {
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
					docs_collection: "doc_chunks_v1".to_string(),
					vector_dim: 4_096,
				},
			},
			providers: Providers {
				embedding: dummy_embedding_provider(),
				rerank: dummy_provider(),
				llm_extractor: dummy_llm_provider(),
			},
			scopes: Scopes {
				allowed: vec!["agent_private".to_string()],
				read_profiles: ReadProfiles {
					private_only: vec!["agent_private".to_string()],
					private_plus_project: vec!["agent_private".to_string()],
					all_scopes: vec!["agent_private".to_string()],
				},
				precedence: ScopePrecedence {
					agent_private: 30,
					project_shared: 20,
					org_shared: 10,
				},
				write_allowed: ScopeWriteAllowed {
					agent_private: true,
					project_shared: true,
					org_shared: true,
				},
			},
			memory: Memory {
				max_notes_per_add_event: 3,
				max_note_chars: 10,
				dup_sim_threshold: 0.9,
				update_sim_threshold: 0.8,
				candidate_k: 10,
				top_k: 5,
				policy: MemoryPolicy::default(),
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
			ranking: test_ranking(),
			lifecycle: Lifecycle {
				ttl_days: TtlDays {
					plan: 1,
					fact: 2,
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

	fn dummy_embedding_provider() -> EmbeddingProviderConfig {
		EmbeddingProviderConfig {
			provider_id: "p".to_string(),
			api_base: "http://localhost".to_string(),
			api_key: "key".to_string(),
			path: "/".to_string(),
			model: "m".to_string(),
			dimensions: 3,
			timeout_ms: 1_000,
			default_headers: serde_json::Map::new(),
		}
	}

	fn dummy_provider() -> ProviderConfig {
		ProviderConfig {
			provider_id: "p".to_string(),
			api_base: "http://localhost".to_string(),
			api_key: "key".to_string(),
			path: "/".to_string(),
			model: "m".to_string(),
			timeout_ms: 1_000,
			default_headers: serde_json::Map::new(),
		}
	}

	fn dummy_llm_provider() -> LlmProviderConfig {
		LlmProviderConfig {
			provider_id: "p".to_string(),
			api_base: "http://localhost".to_string(),
			api_key: "key".to_string(),
			path: "/".to_string(),
			model: "m".to_string(),
			temperature: 0.1,
			timeout_ms: 1_000,
			default_headers: serde_json::Map::new(),
		}
	}

	#[test]
	fn rejects_long_text() {
		let cfg = config();
		let note = NoteInput {
			note_type: "fact".to_string(),
			scope: "agent_private".to_string(),
			text: "12345678901".to_string(),
		};

		assert_eq!(writegate(&note, &cfg), Err(RejectCode::RejectTooLong));
	}

	#[test]
	fn rejects_invalid_type() {
		let cfg = config();
		let note = NoteInput {
			note_type: "other".to_string(),
			scope: "agent_private".to_string(),
			text: "hello".to_string(),
		};

		assert_eq!(writegate(&note, &cfg), Err(RejectCode::RejectInvalidType));
	}

	#[test]
	fn detects_secret_patterns() {
		assert!(contains_secrets("password: hunter2"));
	}
}
