use std::env;

use axum::{
	body::{self, Body},
	http::{Request, StatusCode},
};
use serde_json::{Map, Value};
use tower::util::ServiceExt as _;
use uuid::Uuid;

use elf_api::{routes, state::AppState};
use elf_config::{
	Chunking, Config, EmbeddingProviderConfig, Lifecycle, LlmProviderConfig, Memory, Postgres,
	ProviderConfig, Providers, Qdrant, Ranking, RankingBlend, RankingBlendSegment,
	RankingDeterministic, RankingDeterministicDecay, RankingDeterministicHits,
	RankingDeterministicLexical, RankingDiversity, RankingRetrievalSources, ReadProfiles,
	ScopePrecedence, ScopeWriteAllowed, Scopes, Search, SearchCache, SearchDynamic,
	SearchExpansion, SearchExplain, SearchPrefilter, Security, SecurityAuthKey, SecurityAuthRole,
	Service, Storage, TtlDays,
};
use elf_testkit::TestDatabase;

const TEST_TENANT_ID: &str = "tenant_alpha";
const TEST_PROJECT_ID: &str = "project_alpha";
const TEST_AGENT_A: &str = "a";
const TEST_AGENT_B: &str = "b";

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

fn test_config(dsn: String, qdrant_url: String, collection: String) -> Config {
	Config {
		service: Service {
			http_bind: "127.0.0.1:0".to_string(),
			mcp_bind: "127.0.0.1:0".to_string(),
			admin_bind: "127.0.0.1:0".to_string(),
			log_level: "info".to_string(),
		},
		storage: Storage {
			postgres: Postgres { dsn, pool_max_conns: 1 },
			qdrant: Qdrant { url: qdrant_url, collection, vector_dim: 4_096 },
		},
		providers: Providers {
			embedding: dummy_embedding_provider(),
			rerank: dummy_provider(),
			llm_extractor: dummy_llm_provider(),
		},
		scopes: Scopes {
			allowed: vec![
				"agent_private".to_string(),
				"project_shared".to_string(),
				"org_shared".to_string(),
			],
			read_profiles: ReadProfiles {
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
			policy: Default::default(),
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
			tokenizer_repo: "gpt2".to_string(),
		},
		context: None,
		mcp: None,
	}
}

fn dummy_embedding_provider() -> EmbeddingProviderConfig {
	EmbeddingProviderConfig {
		provider_id: "test".to_string(),
		api_base: "http://127.0.0.1:1".to_string(),
		api_key: "test-key".to_string(),
		path: "/".to_string(),
		model: "test".to_string(),
		dimensions: 4_096,
		timeout_ms: 1_000,
		default_headers: Map::new(),
	}
}

fn dummy_provider() -> ProviderConfig {
	ProviderConfig {
		provider_id: "test".to_string(),
		api_base: "http://127.0.0.1:1".to_string(),
		api_key: "test-key".to_string(),
		path: "/".to_string(),
		model: "test".to_string(),
		timeout_ms: 1_000,
		default_headers: Map::new(),
	}
}

fn dummy_llm_provider() -> LlmProviderConfig {
	LlmProviderConfig {
		provider_id: "test".to_string(),
		api_base: "http://127.0.0.1:1".to_string(),
		api_key: "test-key".to_string(),
		path: "/".to_string(),
		model: "test".to_string(),
		temperature: 0.1,
		timeout_ms: 1_000,
		default_headers: Map::new(),
	}
}

async fn test_env() -> Option<(TestDatabase, String, String)> {
	let base_dsn = match elf_testkit::env_dsn() {
		Some(value) => value,
		None => {
			eprintln!("Skipping HTTP tests; set ELF_PG_DSN to run this test.");

			return None;
		},
	};
	let qdrant_url = match env::var("ELF_QDRANT_GRPC_URL").or_else(|_| env::var("ELF_QDRANT_URL")) {
		Ok(value) => value,
		Err(_) => {
			eprintln!(
				"Skipping HTTP tests; set ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run this test."
			);

			return None;
		},
	};
	let test_db = TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");
	let collection = test_db.collection_name("elf_http");

	Some((test_db, qdrant_url, collection))
}

async fn insert_note(
	state: &AppState,
	note_id: Uuid,
	note_scope: &str,
	note_agent: &str,
	note_text: &str,
) {
	sqlx::query(
		"INSERT INTO memory_notes (
			note_id,
			tenant_id,
			project_id,
			agent_id,
			scope,
			type,
			key,
			text,
			importance,
			confidence,
			status,
			created_at,
			updated_at,
			expires_at,
			embedding_version,
			source_ref
		) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, now(), now(), NULL, $12, $13)",
	)
	.bind(note_id)
	.bind(TEST_TENANT_ID)
	.bind(TEST_PROJECT_ID)
	.bind(note_agent)
	.bind(note_scope)
	.bind("fact")
	.bind(None::<String>)
	.bind(note_text)
	.bind(0.7_f32)
	.bind(0.9_f32)
	.bind("active")
	.bind("v2-test")
	.bind(serde_json::json!({ "source": "integration-test" }))
	.execute(&state.service.db.pool)
	.await
	.expect("Failed to seed memory note.");
}

async fn insert_project_scope_grant(
	state: &AppState,
	owner_agent_id: &str,
	granter_agent_id: &str,
) {
	sqlx::query(
		"INSERT INTO memory_space_grants (
			grant_id,
			tenant_id,
			project_id,
			scope,
			space_owner_agent_id,
			grantee_kind,
			grantee_agent_id,
			granted_by_agent_id
		) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
	)
	.bind(Uuid::new_v4())
	.bind(TEST_TENANT_ID)
	.bind(TEST_PROJECT_ID)
	.bind("project_shared")
	.bind(owner_agent_id)
	.bind("project")
	.bind(None::<String>)
	.bind(granter_agent_id)
	.execute(&state.service.db.pool)
	.await
	.expect("Failed to seed project scope grant.");
}

async fn active_project_grant_count(state: &AppState, owner_agent_id: &str) -> i64 {
	sqlx::query_scalar(
		"SELECT COUNT(*) FROM memory_space_grants \
		WHERE tenant_id = $1 AND project_id = $2 AND scope = 'project_shared' \
		AND space_owner_agent_id = $3 AND grantee_kind = 'project' AND revoked_at IS NULL",
	)
	.bind(TEST_TENANT_ID)
	.bind(TEST_PROJECT_ID)
	.bind(owner_agent_id)
	.fetch_one(&state.service.db.pool)
	.await
	.expect("Failed to query project grant count.")
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn sharing_visibility_requires_explicit_project_grant() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else {
		return;
	};
	let config = test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state.clone());
	let note_id = Uuid::new_v4();

	insert_note(&state, note_id, "project_shared", TEST_AGENT_A, "Fact: shared note without grant")
		.await;

	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("GET")
				.uri("/v2/notes?scope=project_shared")
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_B)
				.body(Body::empty())
				.expect("Failed to build list request."),
		)
		.await
		.expect("Failed to call notes list.");

	assert_eq!(response.status(), StatusCode::OK);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read list response body.");
	let list_json: Value = serde_json::from_slice(&body).expect("Failed to parse list response.");

	assert_eq!(list_json["items"].as_array().expect("Missing items array.").len(), 0);

	let note_response = app
		.clone()
		.oneshot(
			Request::builder()
				.uri(format!("/v2/notes/{note_id}"))
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_B)
				.body(Body::empty())
				.expect("Failed to build get request."),
		)
		.await
		.expect("Failed to call notes get.");

	assert_eq!(note_response.status(), StatusCode::BAD_REQUEST);

	let body = body::to_bytes(note_response.into_body(), usize::MAX)
		.await
		.expect("Failed to read get response body.");
	let note_json: Value = serde_json::from_slice(&body).expect("Failed to parse get response.");

	assert_eq!(note_json["error_code"], "INVALID_REQUEST");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn sharing_project_grant_enables_agent_access_to_shared_note() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else {
		return;
	};
	let config = test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state.clone());
	let note_id = Uuid::new_v4();

	insert_note(
		&state,
		note_id,
		"project_shared",
		TEST_AGENT_A,
		"Fact: shared note with explicit grant.",
	)
	.await;
	insert_project_scope_grant(&state, TEST_AGENT_A, TEST_AGENT_A).await;

	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("GET")
				.uri("/v2/notes?scope=project_shared")
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_B)
				.body(Body::empty())
				.expect("Failed to build list request."),
		)
		.await
		.expect("Failed to call notes list.");

	assert_eq!(response.status(), StatusCode::OK);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read list response body.");
	let list_json: Value = serde_json::from_slice(&body).expect("Failed to parse list response.");
	let items = list_json["items"].as_array().expect("Missing items array.");

	assert_eq!(items.len(), 1);
	assert_eq!(items[0]["note_id"], note_id.to_string());

	let note_response = app
		.clone()
		.oneshot(
			Request::builder()
				.uri(format!("/v2/notes/{note_id}"))
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_B)
				.body(Body::empty())
				.expect("Failed to build get request."),
		)
		.await
		.expect("Failed to call notes get.");

	assert_eq!(note_response.status(), StatusCode::OK);

	let body = body::to_bytes(note_response.into_body(), usize::MAX)
		.await
		.expect("Failed to read get response body.");
	let note_json: Value = serde_json::from_slice(&body).expect("Failed to parse get response.");

	assert_eq!(note_json["note_id"], note_id.to_string());
	assert_eq!(note_json["scope"], "project_shared");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn sharing_publish_creates_scope_and_grant_visibility() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else {
		return;
	};
	let config = test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state.clone());
	let note_id = Uuid::new_v4();

	insert_note(
		&state,
		note_id,
		"agent_private",
		TEST_AGENT_A,
		"Fact: private note for publish test.",
	)
	.await;

	let initial_grant_count = active_project_grant_count(&state, TEST_AGENT_A).await;

	assert_eq!(initial_grant_count, 0);

	let publish_payload = serde_json::json!({"space":"team_shared"}).to_string();
	let publish_response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri(format!("/v2/notes/{note_id}/publish"))
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_A)
				.header("content-type", "application/json")
				.body(Body::from(publish_payload))
				.expect("Failed to build publish request."),
		)
		.await
		.expect("Failed to call note publish.");

	assert_eq!(publish_response.status(), StatusCode::OK);

	let publish_body = body::to_bytes(publish_response.into_body(), usize::MAX)
		.await
		.expect("Failed to read publish response body.");
	let publish_json: Value =
		serde_json::from_slice(&publish_body).expect("Failed to parse publish response.");

	assert_eq!(publish_json["note_id"], note_id.to_string());
	assert_eq!(publish_json["space"], "team_shared");

	let after_grant_count = active_project_grant_count(&state, TEST_AGENT_A).await;

	assert_eq!(after_grant_count, 1);

	let list_response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("GET")
				.uri("/v2/notes?scope=project_shared")
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_B)
				.body(Body::empty())
				.expect("Failed to build list request."),
		)
		.await
		.expect("Failed to call notes list.");

	assert_eq!(list_response.status(), StatusCode::OK);

	let list_body = body::to_bytes(list_response.into_body(), usize::MAX)
		.await
		.expect("Failed to read list response body.");
	let list_json: Value =
		serde_json::from_slice(&list_body).expect("Failed to parse list response.");
	let items = list_json["items"].as_array().expect("Missing items array.");

	assert_eq!(items.len(), 1);
	assert_eq!(items[0]["note_id"], note_id.to_string());

	let get_response = app
		.clone()
		.oneshot(
			Request::builder()
				.uri(format!("/v2/notes/{note_id}"))
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_B)
				.body(Body::empty())
				.expect("Failed to build get request."),
		)
		.await
		.expect("Failed to call notes get.");

	assert_eq!(get_response.status(), StatusCode::OK);

	let get_body = body::to_bytes(get_response.into_body(), usize::MAX)
		.await
		.expect("Failed to read get response body.");
	let get_json: Value = serde_json::from_slice(&get_body).expect("Failed to parse get response.");

	assert_eq!(get_json["note_id"], note_id.to_string());
	assert_eq!(get_json["scope"], "project_shared");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn sharing_revoke_project_grant_removes_visibility() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else {
		return;
	};
	let config = test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state.clone());
	let note_id = Uuid::new_v4();

	insert_note(
		&state,
		note_id,
		"project_shared",
		TEST_AGENT_A,
		"Fact: shared note for revoke test.",
	)
	.await;
	insert_project_scope_grant(&state, TEST_AGENT_A, TEST_AGENT_A).await;

	let grant_count_before = active_project_grant_count(&state, TEST_AGENT_A).await;

	assert_eq!(grant_count_before, 1);

	let list_before = app
		.clone()
		.oneshot(
			Request::builder()
				.method("GET")
				.uri("/v2/notes?scope=project_shared")
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_B)
				.body(Body::empty())
				.expect("Failed to build list request."),
		)
		.await
		.expect("Failed to call notes list.");
	let list_before_body = body::to_bytes(list_before.into_body(), usize::MAX)
		.await
		.expect("Failed to read list response body.");
	let list_before_json: Value =
		serde_json::from_slice(&list_before_body).expect("Failed to parse list response.");

	assert_eq!(list_before_json["items"].as_array().expect("Missing items array.").len(), 1);

	let revoke_payload = serde_json::json!({"grantee_kind":"project"}).to_string();
	let revoke_response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/spaces/team_shared/grants/revoke")
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_A)
				.header("content-type", "application/json")
				.body(Body::from(revoke_payload))
				.expect("Failed to build revoke request."),
		)
		.await
		.expect("Failed to call grant revoke.");

	assert_eq!(revoke_response.status(), StatusCode::OK);

	let grant_count_after = active_project_grant_count(&state, TEST_AGENT_A).await;

	assert_eq!(grant_count_after, 0);

	let list_after = app
		.clone()
		.oneshot(
			Request::builder()
				.method("GET")
				.uri("/v2/notes?scope=project_shared")
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_B)
				.body(Body::empty())
				.expect("Failed to build list request."),
		)
		.await
		.expect("Failed to call notes list.");

	assert_eq!(list_after.status(), StatusCode::OK);

	let list_after_body = body::to_bytes(list_after.into_body(), usize::MAX)
		.await
		.expect("Failed to read list response body.");
	let list_after_json: Value =
		serde_json::from_slice(&list_after_body).expect("Failed to parse list response.");

	assert_eq!(list_after_json["items"].as_array().expect("Missing items array.").len(), 0);

	let get_after = app
		.oneshot(
			Request::builder()
				.uri(format!("/v2/notes/{note_id}"))
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_B)
				.body(Body::empty())
				.expect("Failed to build get request."),
		)
		.await
		.expect("Failed to call notes get.");

	assert_eq!(get_after.status(), StatusCode::BAD_REQUEST);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
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
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
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
	let json: Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["error_code"], "NON_ENGLISH_INPUT");
	assert_eq!(json["fields"][0], "$.notes[0].text");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn rejects_cjk_in_add_event() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else { return };
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
	let json: Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["error_code"], "NON_ENGLISH_INPUT");
	assert_eq!(json["fields"][0], "$.messages[0].content");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
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

	for endpoint in ["/v2/search/quick", "/v2/search/planned"] {
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.method("POST")
					.uri(endpoint)
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
		let json: Value = serde_json::from_slice(&body).expect("Failed to parse response.");

		assert_eq!(json["error_code"], "NON_ENGLISH_INPUT");
		assert_eq!(json["fields"][0], "$.query");
	}

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn static_keys_requires_bearer_header() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else {
		return;
	};
	let mut config = test_config(test_db.dsn().to_string(), qdrant_url, collection);

	config.security.auth_mode = "static_keys".to_string();
	config.security.auth_keys = vec![SecurityAuthKey {
		token_id: "k1".to_string(),
		token: "secret".to_string(),
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: Some("a".to_string()),
		read_profile: "private_plus_project".to_string(),
		role: SecurityAuthRole::User,
	}];

	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);
	let no_auth = app
		.clone()
		.oneshot(Request::builder().uri("/health").body(Body::empty()).expect("build request"))
		.await
		.expect("call /health without auth");

	assert_eq!(no_auth.status(), StatusCode::UNAUTHORIZED);

	let non_bearer_auth = app
		.clone()
		.oneshot(
			Request::builder()
				.uri("/health")
				.header("Authorization", "Basic secret")
				.body(Body::empty())
				.expect("build non-bearer auth request"),
		)
		.await
		.expect("call /health with non-bearer auth");

	assert_eq!(non_bearer_auth.status(), StatusCode::UNAUTHORIZED);

	let bearer_auth = app
		.oneshot(
			Request::builder()
				.uri("/health")
				.header("Authorization", "Bearer secret")
				.body(Body::empty())
				.expect("build bearer auth request"),
		)
		.await
		.expect("call /health with bearer auth");

	assert_eq!(bearer_auth.status(), StatusCode::OK);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn global_graph_predicate_write_requires_super_admin() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else {
		return;
	};
	let mut config = test_config(test_db.dsn().to_string(), qdrant_url, collection);

	config.security.auth_mode = "static_keys".to_string();
	config.security.auth_keys = vec![
		SecurityAuthKey {
			token_id: "admin".to_string(),
			token: "admin-token".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::Admin,
		},
		SecurityAuthKey {
			token_id: "super".to_string(),
			token: "super-token".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::SuperAdmin,
		},
	];

	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::admin_router(state.clone());
	let predicate_id = Uuid::new_v4();

	sqlx::query(
		"\
	INSERT INTO graph_predicates (
		predicate_id,
		scope_key,
		tenant_id,
		project_id,
		canonical,
		canonical_norm,
		cardinality,
		status,
		created_at,
		updated_at
	)
	VALUES ($1, '__global__', NULL, NULL, 'global_test', 'global_test', 'multi', 'pending', now(), now())",
	)
	.bind(predicate_id)
	.execute(&state.service.db.pool)
	.await
	.expect("Failed to insert global predicate.");

	let payload = serde_json::json!({ "status": "active" });
	let response_admin = app
		.clone()
		.oneshot(
			Request::builder()
				.method("PATCH")
				.uri(format!("/v2/admin/graph/predicates/{predicate_id}"))
				.header("Authorization", "Bearer admin-token")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call admin graph predicate patch (admin).");

	assert_eq!(response_admin.status(), StatusCode::FORBIDDEN);

	let body = body::to_bytes(response_admin.into_body(), usize::MAX)
		.await
		.expect("Failed to read response body.");
	let json: Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["error_code"], "SCOPE_DENIED");

	let response_super = app
		.oneshot(
			Request::builder()
				.method("PATCH")
				.uri(format!("/v2/admin/graph/predicates/{predicate_id}"))
				.header("Authorization", "Bearer super-token")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call admin graph predicate patch (super_admin).");

	assert_eq!(response_super.status(), StatusCode::OK);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
