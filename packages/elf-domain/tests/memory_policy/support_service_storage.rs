use crate::support::{
	support_lifecycle, support_memory, support_providers, support_ranking, support_scopes,
	support_search,
};
use elf_config::{Chunking, Config, Postgres, Qdrant, Service, Storage};

pub(crate) fn memory_policy_default_config() -> Config {
	Config {
		service: memory_policy_service_config(),
		storage: memory_policy_storage_config(),
		providers: support_providers::memory_policy_providers_config(),
		scopes: support_scopes::memory_policy_scopes_config(),
		memory: support_memory::memory_policy_memory_config(),
		search: support_search::memory_policy_search_config(),
		ranking: support_ranking::memory_policy_ranking_config(),
		lifecycle: support_lifecycle::memory_policy_lifecycle_config(),
		security: support_lifecycle::memory_policy_security_config(),
		chunking: memory_policy_chunking_config(),
		context: None,
		mcp: None,
	}
}

fn memory_policy_service_config() -> Service {
	Service {
		http_bind: "127.0.0.1:8080".to_string(),
		mcp_bind: "127.0.0.1:8082".to_string(),
		admin_bind: "127.0.0.1:8081".to_string(),
		log_level: "info".to_string(),
	}
}

fn memory_policy_storage_config() -> Storage {
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

fn memory_policy_chunking_config() -> Chunking {
	Chunking {
		enabled: true,
		max_tokens: 512,
		overlap_tokens: 128,
		tokenizer_repo: "REPLACE_ME".to_string(),
	}
}
