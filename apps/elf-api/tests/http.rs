#[path = "../src/routes.rs"] mod routes;
#[path = "../src/state.rs"] mod state;

use axum::{
	body::Body,
	http::{Request, StatusCode},
};
use sqlx::Connection;
use tower::util::ServiceExt;

const TEST_DB_LOCK_KEY: i64 = 0x454C4601;

struct DbLock {
	_conn: sqlx::PgConnection,
}

async fn acquire_db_lock(dsn: &str) -> DbLock {
	let mut conn = sqlx::PgConnection::connect(dsn).await.expect("Failed to connect for DB lock.");
	sqlx::query("SELECT pg_advisory_lock($1)")
		.bind(TEST_DB_LOCK_KEY)
		.execute(&mut conn)
		.await
		.expect("Failed to acquire DB lock.");
	DbLock { _conn: conn }
}

fn test_env() -> Option<(String, String)> {
	let dsn = match std::env::var("ELF_PG_DSN") {
		Ok(value) => value,
		Err(_) => {
			eprintln!("Skipping HTTP tests; set ELF_PG_DSN to run this test.");
			return None;
		},
	};
	let qdrant_url = match std::env::var("ELF_QDRANT_URL") {
		Ok(value) => value,
		Err(_) => {
			eprintln!("Skipping HTTP tests; set ELF_QDRANT_URL to run this test.");
			return None;
		},
	};
	Some((dsn, qdrant_url))
}

fn test_config(dsn: String, qdrant_url: String) -> elf_config::Config {
	elf_config::Config {
		service: elf_config::Service {
			http_bind: "127.0.0.1:0".to_string(),
			mcp_bind: "127.0.0.1:0".to_string(),
			admin_bind: "127.0.0.1:0".to_string(),
			log_level: "info".to_string(),
		},
		storage: elf_config::Storage {
			postgres: elf_config::Postgres { dsn, pool_max_conns: 1 },
			qdrant: elf_config::Qdrant {
				url: qdrant_url,
				collection: "elf_notes".to_string(),
				vector_dim: 3,
			},
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
		ranking: elf_config::Ranking { recency_tau_days: 60.0, tie_breaker_weight: 0.1 },
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
	}
}

fn dummy_embedding_provider() -> elf_config::EmbeddingProviderConfig {
	elf_config::EmbeddingProviderConfig {
		provider_id: "test".to_string(),
		base_url: "http://127.0.0.1:1".to_string(),
		api_key: "test-key".to_string(),
		path: "/".to_string(),
		model: "test".to_string(),
		dimensions: 3,
		timeout_ms: 1000,
		default_headers: serde_json::Map::new(),
	}
}

fn dummy_provider() -> elf_config::ProviderConfig {
	elf_config::ProviderConfig {
		provider_id: "test".to_string(),
		base_url: "http://127.0.0.1:1".to_string(),
		api_key: "test-key".to_string(),
		path: "/".to_string(),
		model: "test".to_string(),
		timeout_ms: 1000,
		default_headers: serde_json::Map::new(),
	}
}

fn dummy_llm_provider() -> elf_config::LlmProviderConfig {
	elf_config::LlmProviderConfig {
		provider_id: "test".to_string(),
		base_url: "http://127.0.0.1:1".to_string(),
		api_key: "test-key".to_string(),
		path: "/".to_string(),
		model: "test".to_string(),
		temperature: 0.1,
		timeout_ms: 1000,
		default_headers: serde_json::Map::new(),
	}
}

#[tokio::test]
async fn health_ok() {
	let Some((dsn, qdrant_url)) = test_env() else {
		return;
	};
	let _lock = acquire_db_lock(&dsn).await;
	let config = test_config(dsn, qdrant_url);
	let state = state::AppState::new(config).await.expect("Failed to initialize app state.");
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
}

#[tokio::test]
async fn rejects_cjk_in_add_note() {
	let Some((dsn, qdrant_url)) = test_env() else {
		return;
	};
	let _lock = acquire_db_lock(&dsn).await;
	let config = test_config(dsn, qdrant_url);
	let state = state::AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);
	let payload = serde_json::json!({
		"tenant_id": "t",
		"project_id": "p",
		"agent_id": "a",
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
				.uri("/v1/memory/add_note")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call add_note.");

	assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
	let body = axum::body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read response body.");
	let json: serde_json::Value = serde_json::from_slice(&body).expect("Failed to parse response.");
	assert_eq!(json["error_code"], "NON_ENGLISH_INPUT");
	assert_eq!(json["fields"][0], "$.notes[0].text");
}

#[tokio::test]
async fn rejects_cjk_in_add_event() {
	let Some((dsn, qdrant_url)) = test_env() else {
		return;
	};
	let _lock = acquire_db_lock(&dsn).await;
	let config = test_config(dsn, qdrant_url);
	let state = state::AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);
	let payload = serde_json::json!({
		"tenant_id": "t",
		"project_id": "p",
		"agent_id": "a",
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
				.uri("/v1/memory/add_event")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call add_event.");

	assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
	let body = axum::body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read response body.");
	let json: serde_json::Value = serde_json::from_slice(&body).expect("Failed to parse response.");
	assert_eq!(json["error_code"], "NON_ENGLISH_INPUT");
	assert_eq!(json["fields"][0], "$.messages[0].content");
}

#[tokio::test]
async fn rejects_cjk_in_search() {
	let Some((dsn, qdrant_url)) = test_env() else {
		return;
	};
	let _lock = acquire_db_lock(&dsn).await;
	let config = test_config(dsn, qdrant_url);
	let state = state::AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);
	let payload = serde_json::json!({
		"tenant_id": "t",
		"project_id": "p",
		"agent_id": "a",
		"read_profile": "private_only",
		"query": "안녕하세요",
		"top_k": 5,
		"candidate_k": 10
	});

	let response = app
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v1/memory/search")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call search.");

	assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
	let body = axum::body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read response body.");
	let json: serde_json::Value = serde_json::from_slice(&body).expect("Failed to parse response.");
	assert_eq!(json["error_code"], "NON_ENGLISH_INPUT");
	assert_eq!(json["fields"][0], "$.query");
}
