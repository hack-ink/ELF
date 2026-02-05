use regex::Regex;

use crate::cjk;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectCode {
	RejectCjk,
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

pub fn writegate(note: &NoteInput, cfg: &elf_config::Config) -> Result<(), RejectCode> {
	if note.text.trim().is_empty() {
		return Err(RejectCode::RejectEmpty);
	}
	if cjk::contains_cjk(&note.text) {
		return Err(RejectCode::RejectCjk);
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

fn scope_write_allowed(cfg: &elf_config::Config, scope: &str) -> bool {
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
	use super::*;

	fn config() -> elf_config::Config {
		elf_config::Config {
			service: elf_config::Service {
				http_bind: "127.0.0.1:8080".to_string(),
				mcp_bind: "127.0.0.1:8082".to_string(),
				admin_bind: "127.0.0.1:8081".to_string(),
				log_level: "info".to_string(),
			},
			storage: elf_config::Storage {
				postgres: elf_config::Postgres {
					dsn: "postgres://user:pass@localhost/db".to_string(),
					pool_max_conns: 1,
				},
				qdrant: elf_config::Qdrant {
					url: "http://localhost".to_string(),
					collection: "mem_notes_v1".to_string(),
					vector_dim: 3,
				},
			},
			providers: elf_config::Providers {
				embedding: dummy_embedding_provider(),
				rerank: dummy_provider(),
				llm_extractor: dummy_llm_provider(),
			},
			scopes: elf_config::Scopes {
				allowed: vec!["agent_private".to_string()],
				read_profiles: elf_config::ReadProfiles {
					private_only: vec!["agent_private".to_string()],
					private_plus_project: vec!["agent_private".to_string()],
					all_scopes: vec!["agent_private".to_string()],
				},
				precedence: elf_config::ScopePrecedence {
					agent_private: 30,
					project_shared: 20,
					org_shared: 10,
				},
				write_allowed: elf_config::ScopeWriteAllowed {
					agent_private: true,
					project_shared: true,
					org_shared: true,
				},
			},
			memory: elf_config::Memory {
				max_notes_per_add_event: 3,
				max_note_chars: 10,
				dup_sim_threshold: 0.9,
				update_sim_threshold: 0.8,
				candidate_k: 10,
				top_k: 5,
			},
			search: elf_config::Search {
				expansion: elf_config::SearchExpansion {
					mode: "off".to_string(),
					max_queries: 4,
					include_original: true,
				},
				dynamic: elf_config::SearchDynamic { min_candidates: 10, min_top_score: 0.12 },
				prefilter: elf_config::SearchPrefilter { max_candidates: 0 },
				cache: elf_config::SearchCache {
					enabled: true,
					expansion_ttl_days: 7,
					rerank_ttl_days: 7,
					max_payload_bytes: Some(262_144),
					expansion_version: "v1".to_string(),
					rerank_version: "v1".to_string(),
				},
				explain: elf_config::SearchExplain { retention_days: 7 },
			},
			ranking: elf_config::Ranking { recency_tau_days: 60.0, tie_breaker_weight: 0.1 },
			lifecycle: elf_config::Lifecycle {
				ttl_days: elf_config::TtlDays {
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
			security: elf_config::Security {
				bind_localhost_only: true,
				reject_cjk: true,
				redact_secrets_on_write: true,
				evidence_min_quotes: 1,
				evidence_max_quotes: 2,
				evidence_max_quote_chars: 320,
			},
			chunking: elf_config::Chunking {
				enabled: true,
				max_tokens: 512,
				overlap_tokens: 128,
				tokenizer_repo: None,
			},
		}
	}

	fn dummy_embedding_provider() -> elf_config::EmbeddingProviderConfig {
		elf_config::EmbeddingProviderConfig {
			provider_id: "p".to_string(),
			api_base: "http://localhost".to_string(),
			api_key: "key".to_string(),
			path: "/".to_string(),
			model: "m".to_string(),
			dimensions: 3,
			timeout_ms: 1000,
			default_headers: serde_json::Map::new(),
		}
	}

	fn dummy_provider() -> elf_config::ProviderConfig {
		elf_config::ProviderConfig {
			provider_id: "p".to_string(),
			api_base: "http://localhost".to_string(),
			api_key: "key".to_string(),
			path: "/".to_string(),
			model: "m".to_string(),
			timeout_ms: 1000,
			default_headers: serde_json::Map::new(),
		}
	}

	fn dummy_llm_provider() -> elf_config::LlmProviderConfig {
		elf_config::LlmProviderConfig {
			provider_id: "p".to_string(),
			api_base: "http://localhost".to_string(),
			api_key: "key".to_string(),
			path: "/".to_string(),
			model: "m".to_string(),
			temperature: 0.1,
			timeout_ms: 1000,
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
