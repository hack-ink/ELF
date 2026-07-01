use crate::memory_policy::tests::support::{lifecycle, memory, providers, ranking, scopes, search};
use elf_config::{Chunking, Config, Postgres, Qdrant, Service, Storage};

pub(crate) fn test_default_config() -> Config {
	Config {
		service: test_service_config(),
		storage: test_storage_config(),
		providers: providers::test_providers_config(),
		scopes: scopes::test_scopes_config(),
		memory: memory::test_memory_config(),
		search: search::test_search_config(),
		ranking: ranking::test_ranking_config(),
		lifecycle: lifecycle::test_lifecycle_config(),
		security: lifecycle::test_security_config(),
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

fn test_chunking_config() -> Chunking {
	Chunking {
		enabled: true,
		max_tokens: 512,
		overlap_tokens: 128,
		tokenizer_repo: "REPLACE_ME".to_string(),
	}
}
