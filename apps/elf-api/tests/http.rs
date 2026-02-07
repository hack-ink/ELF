use std::env;

use axum::{
	body::{self, Body},
	http::{Request, StatusCode},
};
use serde_json::Map;
use tower::util::ServiceExt;

use elf_api::{routes, state::AppState};
use elf_testkit::TestDatabase;

fn test_config(dsn: String, qdrant_url: String, collection: String) -> elf_config::Config {
	elf_config::Config {
		service: elf_config::Service {
			http_bind: "127.0.0.1:0".to_string(),
			mcp_bind: "127.0.0.1:0".to_string(),
			admin_bind: "127.0.0.1:0".to_string(),
			log_level: "info".to_string(),
		},
		storage: elf_config::Storage {
			postgres: elf_config::Postgres { dsn, pool_max_conns: 1 },
			qdrant: elf_config::Qdrant { url: qdrant_url, collection, vector_dim: 3 },
		},
		providers: elf_config::Providers {
			embedding: dummy_embedding_provider(),
			rerank: dummy_provider(),
			llm_extractor: dummy_llm_provider(),
		},
		scopes: elf_config::Scopes {
			allowed: vec![
				"agent_private".to_string(),
				"project_shared".to_string(),
				"org_shared".to_string(),
			],
			read_profiles: elf_config::ReadProfiles {
				private_only: vec!["agent_private".to_string()],
				private_plus_project: vec![
					"agent_private".to_string(),
					"project_shared".to_string(),
				],
				all_scopes: vec![
					"agent_private".to_string(),
					"project_shared".to_string(),
					"org_shared".to_string(),
				],
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
			max_note_chars: 240,
			dup_sim_threshold: 0.92,
			update_sim_threshold: 0.85,
			candidate_k: 60,
			top_k: 12,
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
			},
			explain: elf_config::SearchExplain { retention_days: 7 },
		},
		ranking: elf_config::Ranking {
			recency_tau_days: 60.0,
			tie_breaker_weight: 0.1,
			blend: Default::default(),
		},
		lifecycle: elf_config::Lifecycle {
			ttl_days: elf_config::TtlDays {
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
		context: None,
		mcp: None,
	}
}

fn dummy_embedding_provider() -> elf_config::EmbeddingProviderConfig {
	elf_config::EmbeddingProviderConfig {
		provider_id: "test".to_string(),
		api_base: "http://127.0.0.1:1".to_string(),
		api_key: "test-key".to_string(),
		path: "/".to_string(),
		model: "test".to_string(),
		dimensions: 3,
		timeout_ms: 1000,
		default_headers: Map::new(),
	}
}

fn dummy_provider() -> elf_config::ProviderConfig {
	elf_config::ProviderConfig {
		provider_id: "test".to_string(),
		api_base: "http://127.0.0.1:1".to_string(),
		api_key: "test-key".to_string(),
		path: "/".to_string(),
		model: "test".to_string(),
		timeout_ms: 1000,
		default_headers: Map::new(),
	}
}

fn dummy_llm_provider() -> elf_config::LlmProviderConfig {
	elf_config::LlmProviderConfig {
		provider_id: "test".to_string(),
		api_base: "http://127.0.0.1:1".to_string(),
		api_key: "test-key".to_string(),
		path: "/".to_string(),
		model: "test".to_string(),
		temperature: 0.1,
		timeout_ms: 1000,
		default_headers: Map::new(),
	}
}

async fn test_env() -> Option<(elf_testkit::TestDatabase, String, String)> {
	let base_dsn = match elf_testkit::env_dsn() {
		Some(value) => value,
		None => {
			eprintln!("Skipping HTTP tests; set ELF_PG_DSN to run this test.");

			return None;
		},
	};
	let qdrant_url = match env::var("ELF_QDRANT_URL") {
		Ok(value) => value,
		Err(_) => {
			eprintln!("Skipping HTTP tests; set ELF_QDRANT_URL to run this test.");

			return None;
		},
	};
	let test_db = TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");
	let collection = test_db.collection_name("elf_http");

	Some((test_db, qdrant_url, collection))
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn health_ok() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else {
		return;
	};
	let config = test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state.clone());
	let _ = routes::admin_router(state);
	let response = app
		.oneshot(
			Request::builder()
				.uri("/health")
				.body(Body::empty())
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call /health.");

	assert_eq!(response.status(), StatusCode::OK);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_cjk_in_add_note() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else {
		return;
	};
	let config = test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);
	let payload = serde_json::json!({
		"scope": "agent_private",
		"notes": [{
			"type": "fact",
			"key": null,
			"text": "你好",
			"importance": 0.5,
			"confidence": 0.9,
			"ttl_days": null,
			"source_ref": {}
		}]
	});
	let response = app
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/notes/ingest")
				.header("X-ELF-Tenant-Id", "t")
				.header("X-ELF-Project-Id", "p")
				.header("X-ELF-Agent-Id", "a")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call add_note.");

	assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read response body.");
	let json: serde_json::Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["error_code"], "NON_ENGLISH_INPUT");
	assert_eq!(json["fields"][0], "$.notes[0].text");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_cjk_in_add_event() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else {
		return;
	};
	let config = test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);
	let payload = serde_json::json!({
		"scope": "agent_private",
		"dry_run": true,
		"messages": [{
			"role": "user",
			"content": "こんにちは"
		}]
	});
	let response = app
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/events/ingest")
				.header("X-ELF-Tenant-Id", "t")
				.header("X-ELF-Project-Id", "p")
				.header("X-ELF-Agent-Id", "a")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call add_event.");

	assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read response body.");
	let json: serde_json::Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["error_code"], "NON_ENGLISH_INPUT");
	assert_eq!(json["fields"][0], "$.messages[0].content");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_cjk_in_search() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else {
		return;
	};
	let config = test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);
	let payload = serde_json::json!({
		"query": "안녕하세요",
		"top_k": 5,
		"candidate_k": 10
	});
	let response = app
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/searches")
				.header("X-ELF-Tenant-Id", "t")
				.header("X-ELF-Project-Id", "p")
				.header("X-ELF-Agent-Id", "a")
				.header("X-ELF-Read-Profile", "private_only")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call search.");

	assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read response body.");
	let json: serde_json::Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["error_code"], "NON_ENGLISH_INPUT");
	assert_eq!(json["fields"][0], "$.query");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
