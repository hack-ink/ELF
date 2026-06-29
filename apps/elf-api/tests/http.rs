#![allow(unused_crate_dependencies)]

//! End-to-end HTTP integration tests for the ELF API app.

#[path = "http/auth_admin.rs"] mod auth_admin;
#[path = "http/contract.rs"] mod contract;
#[path = "http/request_validation.rs"] mod request_validation;
#[path = "http/sharing.rs"] mod sharing;

use std::{collections::HashMap, env};

use axum::{
	Router,
	body::{self, Body},
	http::{Request, Response, StatusCode},
};
use qdrant_client::{
	Payload,
	qdrant::{Document, PointStruct, UpsertPointsBuilder, Vector},
};
use serde_json::Map;
use tower::util::ServiceExt;
use tracing::Level;
use uuid::Uuid;

use elf_api::{
	routes::{self, OPENAPI_JSON_PATH},
	state::AppState,
};
use elf_config::{
	Chunking, Config, EmbeddingProviderConfig, Lifecycle, LlmProviderConfig, Memory, MemoryPolicy,
	Postgres, ProviderConfig, Providers, Qdrant, Ranking, RankingBlend, RankingBlendSegment,
	RankingDeterministic, RankingDeterministicDecay, RankingDeterministicHits,
	RankingDeterministicLexical, RankingDiversity, RankingRetrievalSources, ReadProfiles,
	ScopePrecedence, ScopeWriteAllowed, Scopes, Search, SearchCache, SearchDynamic,
	SearchExpansion, SearchExplain, SearchGraphContext, SearchPrefilter, SearchRecursive, Security,
	SecurityAuthKey, SecurityAuthRole, Service, Storage, TtlDays,
};
use elf_storage::qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME};
use elf_testkit::TestDatabase;

const TEST_TENANT_ID: &str = "tenant_alpha";
const TEST_PROJECT_ID: &str = "project_alpha";
const TEST_PROJECT_ID_B: &str = "project_beta";
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
			postgres: Postgres { dsn, pool_max_conns: 4 },
			qdrant: Qdrant {
				url: qdrant_url,
				collection: collection.clone(),
				docs_collection: format!("{collection}_docs"),
				vector_dim: 4_096,
			},
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
			policy: MemoryPolicy { rules: vec![] },
		},
		search: test_search(),
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
			reject_non_english: true,
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

fn test_search() -> Search {
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

fn dummy_embedding_provider() -> EmbeddingProviderConfig {
	EmbeddingProviderConfig {
		provider_id: "local".to_string(),
		api_base: "http://127.0.0.1:1".to_string(),
		api_key: "test-key".to_string(),
		path: "/".to_string(),
		model: "local-hash".to_string(),
		dimensions: 4_096,
		timeout_ms: 1_000,
		default_headers: Map::new(),
	}
}

fn dummy_provider() -> ProviderConfig {
	ProviderConfig {
		provider_id: "local".to_string(),
		api_base: "http://127.0.0.1:1".to_string(),
		api_key: "test-key".to_string(),
		path: "/".to_string(),
		model: "local-token-overlap".to_string(),
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

fn assert_openapi_method(spec: &serde_json::Value, path: &str, method: &str) {
	let operation = spec
		.get("paths")
		.and_then(|paths| paths.get(path))
		.and_then(|path_item| path_item.get(method));

	assert!(operation.is_some(), "Missing OpenAPI operation {method} {path}");
}

fn init_test_tracing() {
	let _ = tracing_subscriber::fmt().with_max_level(Level::ERROR).with_test_writer().try_init();
}

fn context_request(
	method: &str,
	uri: impl AsRef<str>,
	agent_id: &str,
	read_profile: &str,
) -> Request<Body> {
	Request::builder()
		.method(method)
		.uri(uri.as_ref())
		.header("content-type", "application/json")
		.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
		.header("X-ELF-Project-Id", TEST_PROJECT_ID)
		.header("X-ELF-Agent-Id", agent_id)
		.header("X-ELF-Read-Profile", read_profile)
		.body(Body::empty())
		.expect("Failed to build context request.")
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

async fn search_session_count(state: &AppState) -> i64 {
	sqlx::query_scalar("SELECT COUNT(*) FROM search_sessions")
		.fetch_one(&state.service.db.pool)
		.await
		.expect("Failed to count search sessions.")
}

async fn post_admin_json(
	app: &Router,
	uri: impl AsRef<str>,
	agent_id: &str,
	body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
	let request = Request::builder()
		.method("POST")
		.uri(uri.as_ref())
		.header("content-type", "application/json")
		.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
		.header("X-ELF-Project-Id", TEST_PROJECT_ID)
		.header("X-ELF-Agent-Id", agent_id)
		.body(Body::from(body.to_string()))
		.expect("Failed to build admin JSON request.");
	let response = app.clone().oneshot(request).await.expect("Failed to call admin route.");
	let status = response.status();
	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read admin response body.");

	(status, serde_json::from_slice(&body).expect("Failed to parse admin response."))
}

async fn create_core_block(admin_app: &Router, scope: &str, key: &str, content: &str) -> Uuid {
	let payload = serde_json::json!({
		"scope": scope,
		"key": key,
		"title": "Operating context",
		"content": content,
		"source_ref": {
			"schema": "core_block_source/v1",
			"ref": { "issue": "XY-832" }
		}
	});
	let (status, body) =
		post_admin_json(admin_app, "/v2/admin/core-blocks", TEST_AGENT_A, payload).await;

	assert_eq!(status, StatusCode::OK);

	Uuid::parse_str(
		body.pointer("/block/block_id")
			.and_then(serde_json::Value::as_str)
			.expect("Missing core block id."),
	)
	.expect("Invalid core block id.")
}

async fn attach_core_block(
	admin_app: &Router,
	block_id: Uuid,
	target_agent_id: &str,
	read_profile: &str,
) -> (StatusCode, serde_json::Value) {
	let payload = serde_json::json!({
		"target_agent_id": target_agent_id,
		"read_profile": read_profile,
		"reason": "Attach fixture block."
	});
	let uri = format!("/v2/admin/core-blocks/{block_id}/attachments");

	post_admin_json(admin_app, uri, TEST_AGENT_A, payload).await
}

async fn get_core_blocks(app: &Router, agent_id: &str, read_profile: &str) -> serde_json::Value {
	let response = app
		.clone()
		.oneshot(context_request("GET", "/v2/core-blocks", agent_id, read_profile))
		.await
		.expect("Failed to fetch core blocks.");

	assert_eq!(response.status(), StatusCode::OK);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read core blocks response body.");

	serde_json::from_slice(&body).expect("Failed to parse core blocks response.")
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

async fn note_scope_and_project_id(state: &AppState, note_id: Uuid) -> (String, String) {
	let row: (String, String) = sqlx::query_as(
		"SELECT scope, project_id FROM memory_notes WHERE tenant_id = $1 AND note_id = $2",
	)
	.bind(TEST_TENANT_ID)
	.bind(note_id)
	.fetch_one(&state.service.db.pool)
	.await
	.expect("Failed to query note scope and project id.");

	row
}

async fn active_org_shared_project_grant_count(state: &AppState, owner_agent_id: &str) -> i64 {
	sqlx::query_scalar(
		"SELECT COUNT(*) FROM memory_space_grants \
		WHERE tenant_id = $1 AND project_id = '__org__' AND scope = 'org_shared' \
		AND space_owner_agent_id = $2 AND grantee_kind = 'project' AND revoked_at IS NULL",
	)
	.bind(TEST_TENANT_ID)
	.bind(owner_agent_id)
	.fetch_one(&state.service.db.pool)
	.await
	.expect("Failed to query org_shared project grant count.")
}

async fn active_org_shared_project_grant_count_for_project(
	state: &AppState,
	project_id: &str,
	owner_agent_id: &str,
) -> i64 {
	sqlx::query_scalar(
		"SELECT COUNT(*) FROM memory_space_grants \
		WHERE tenant_id = $1 AND project_id = $2 AND scope = 'org_shared' \
		AND space_owner_agent_id = $3 AND grantee_kind = 'project' AND revoked_at IS NULL",
	)
	.bind(TEST_TENANT_ID)
	.bind(project_id)
	.bind(owner_agent_id)
	.fetch_one(&state.service.db.pool)
	.await
	.expect("Failed to query org_shared project grant count for project.")
}

async fn org_shared_note_is_visible_across_projects_fixture()
-> Option<(TestDatabase, Router, AppState, Uuid)> {
	let (test_db, qdrant_url, collection) = test_env().await?;
	let mut config = test_config(test_db.dsn().to_string(), qdrant_url, collection);

	config.security.auth_mode = "static_keys".to_string();
	config.security.auth_keys = vec![
		SecurityAuthKey {
			token_id: "admin-token-id".to_string(),
			token: "admin-token".to_string(),
			tenant_id: TEST_TENANT_ID.to_string(),
			project_id: TEST_PROJECT_ID.to_string(),
			agent_id: Some("admin-agent".to_string()),
			read_profile: "all_scopes".to_string(),
			role: SecurityAuthRole::Admin,
		},
		SecurityAuthKey {
			token_id: "reader-token-id".to_string(),
			token: "reader-token".to_string(),
			tenant_id: TEST_TENANT_ID.to_string(),
			project_id: TEST_PROJECT_ID_B.to_string(),
			agent_id: Some("reader-agent".to_string()),
			read_profile: "all_scopes".to_string(),
			role: SecurityAuthRole::User,
		},
	];

	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state.clone());
	let note_id = Uuid::new_v4();

	insert_note(
		&state,
		note_id,
		"agent_private",
		"admin-agent",
		"Fact: org_shared cross-project visibility.",
	)
	.await;

	Some((test_db, app, state, note_id))
}

async fn list_org_shared_notes_as_reader(app: &Router) -> serde_json::Value {
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("GET")
				.uri("/v2/notes?scope=org_shared")
				.header("Authorization", "Bearer reader-token")
				.body(Body::empty())
				.expect("Failed to build list request."),
		)
		.await
		.expect("Failed to call notes list.");

	assert_eq!(response.status(), StatusCode::OK);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read list response body.");

	serde_json::from_slice(&body).expect("Failed to parse list response.")
}

async fn publish_org_shared_note_as_reader_can_see(scope_app: &Router, note_id: Uuid) {
	let payload = serde_json::json!({ "space": "org_shared" }).to_string();
	let response = scope_app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri(format!("/v2/notes/{note_id}/publish"))
				.header("Authorization", "Bearer admin-token")
				.header("content-type", "application/json")
				.body(Body::from(payload))
				.expect("Failed to build note publish request."),
		)
		.await
		.expect("Failed to call notes publish.");

	assert_eq!(response.status(), StatusCode::OK);
}

async fn assert_note_visible_to_project_reader(
	scope_app: &Router,
	state: &AppState,
	note_id: Uuid,
) {
	let (scope, project_id) = note_scope_and_project_id(state, note_id).await;

	assert_eq!(scope, "org_shared");
	// org_shared note rows live in the synthetic org project, not the request project.
	assert_eq!(project_id, "__org__");

	let org_grant_count = active_org_shared_project_grant_count(state, "admin-agent").await;

	assert!(org_grant_count > 0);

	// org_shared grant rows live in '__org__' as well; they should not be written into the request
	// project.
	let request_project_grant_count =
		active_org_shared_project_grant_count_for_project(state, TEST_PROJECT_ID, "admin-agent")
			.await;

	assert_eq!(request_project_grant_count, 0);

	let list_after_json = list_org_shared_notes_as_reader(scope_app).await;
	let items = list_after_json["items"].as_array().expect("Missing items array.");
	let ids: Vec<&str> = items.iter().filter_map(|item| item["note_id"].as_str()).collect();
	let note_id_str = note_id.to_string();

	assert!(ids.contains(&note_id_str.as_str()));
}

async fn post_with_authorization_and_json_body(
	app: &Router,
	uri: &str,
	auth: &str,
	payload: &str,
	build_expect: &str,
	call_expect: &str,
) -> Response<Body> {
	app.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri(uri)
				.header("Authorization", auth)
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect(build_expect),
		)
		.await
		.expect(call_expect)
}

async fn create_note_for_payload_level_tests(
	app: &Router,
	state: &AppState,
	text: &str,
	source_ref: serde_json::Value,
) -> Uuid {
	init_test_tracing();

	let payload = serde_json::json!({
		"scope": "agent_private",
		"notes": [{
			"type": "fact",
			"key": null,
			"text": text,
			"importance": 0.8,
			"confidence": 0.9,
			"ttl_days": null,
			"source_ref": source_ref,
		}]
	});
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/notes/ingest")
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_A)
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build note ingest request."),
		)
		.await
		.expect("Failed to call note ingest.");
	let status = response.status();
	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read note ingest response body.");

	assert_eq!(
		status,
		StatusCode::OK,
		"Unexpected note ingest status with body: {}",
		String::from_utf8_lossy(&body)
	);

	let json: serde_json::Value =
		serde_json::from_slice(&body).expect("Failed to parse note ingest response.");
	let note_id = json["results"]
		.as_array()
		.expect("Missing results array in note ingest response.")
		.first()
		.and_then(|result| result["note_id"].as_str())
		.expect("Missing note_id in note ingest response.");
	let note_id = Uuid::parse_str(note_id).expect("Invalid note_id in note ingest response.");

	index_note_for_payload_level_tests(state, note_id, text).await;

	note_id
}

async fn index_note_for_payload_level_tests(state: &AppState, note_id: Uuid, text: &str) {
	let chunk_id = Uuid::new_v4();
	let embedding_version = format!(
		"{}:{}:{}",
		state.service.cfg.providers.embedding.provider_id,
		state.service.cfg.providers.embedding.model,
		state.service.cfg.storage.qdrant.vector_dim
	);

	sqlx::query(
		"INSERT INTO memory_note_chunks (
			chunk_id,
			note_id,
			chunk_index,
			start_offset,
			end_offset,
			text,
			embedding_version
		) VALUES ($1, $2, $3, $4, $5, $6, $7)",
	)
	.bind(chunk_id)
	.bind(note_id)
	.bind(0_i32)
	.bind(0_i32)
	.bind(i32::try_from(text.len()).expect("Payload-level test text fits i32 offsets."))
	.bind(text)
	.bind(embedding_version.as_str())
	.execute(&state.service.db.pool)
	.await
	.expect("Failed to seed memory note chunk.");

	let mut payload = Payload::new();

	payload.insert("note_id", note_id.to_string());
	payload.insert("chunk_id", chunk_id.to_string());
	payload.insert("chunk_index", 0_i64);
	payload.insert("start_offset", 0_i64);
	payload.insert("end_offset", i64::try_from(text.len()).expect("Test text fits i64 offsets."));
	payload.insert("tenant_id", TEST_TENANT_ID);
	payload.insert("project_id", TEST_PROJECT_ID);
	payload.insert("agent_id", TEST_AGENT_A);
	payload.insert("scope", "agent_private");
	payload.insert("type", "fact");
	payload.insert("status", "active");
	payload.insert("embedding_version", embedding_version);

	let mut vectors = HashMap::new();

	vectors.insert(
		DENSE_VECTOR_NAME.to_string(),
		Vector::from(vec![0.0_f32; state.service.qdrant.vector_dim as usize]),
	);
	vectors.insert(
		BM25_VECTOR_NAME.to_string(),
		Vector::from(Document::new(text.to_string(), BM25_MODEL)),
	);

	let point = PointStruct::new(chunk_id.to_string(), vectors, payload);

	state
		.service
		.qdrant
		.client
		.upsert_points(
			UpsertPointsBuilder::new(state.service.qdrant.collection.clone(), vec![point])
				.wait(true),
		)
		.await
		.expect("Failed to seed Qdrant point.");
}

async fn insert_note_summary_field(state: &AppState, note_id: Uuid, summary: &str) {
	sqlx::query(
		"INSERT INTO memory_note_fields (field_id, note_id, field_kind, item_index, text) \
		 VALUES ($1, $2, $3, $4, $5)",
	)
	.bind(Uuid::new_v4())
	.bind(note_id)
	.bind("summary")
	.bind(0)
	.bind(summary)
	.execute(&state.service.db.pool)
	.await
	.expect("Failed to insert note summary field.");
}

async fn fetch_search_notes_for_payload_level(
	app: &Router,
	search_id: Uuid,
	note_id: Uuid,
	payload_level: &str,
) -> serde_json::Value {
	let payload = serde_json::json!({
		"note_ids": [note_id],
		"payload_level": payload_level,
		"record_hits": false,
	});
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri(format!("/v2/searches/{search_id}/notes"))
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_A)
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build search notes request."),
		)
		.await
		.expect("Failed to call search notes.");
	let status = response.status();
	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read search notes response body.");

	assert_eq!(
		status,
		StatusCode::OK,
		"Unexpected search notes response: {}",
		String::from_utf8_lossy(&body)
	);

	let json: serde_json::Value =
		serde_json::from_slice(&body).expect("Failed to parse search notes response.");

	json.get("results")
		.and_then(serde_json::Value::as_array)
		.and_then(|results| results.first())
		.and_then(|result| result.get("note"))
		.cloned()
		.expect("Expected note in search notes response.")
}

async fn fetch_admin_search_raw_source_ref(
	app: &Router,
	query: &str,
	payload_level: &str,
) -> serde_json::Value {
	let payload = serde_json::json!({
		"mode": "quick_find",
		"query": query,
		"top_k": 5,
		"candidate_k": 10,
		"payload_level": payload_level,
	});
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/admin/searches/raw")
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_A)
				.header("X-ELF-Read-Profile", "private_only")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build admin search raw request."),
		)
		.await
		.expect("Failed to call admin search raw.");
	let status = response.status();
	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read admin search raw response body.");

	assert_eq!(
		status,
		StatusCode::OK,
		"Unexpected admin search raw status with body: {}",
		String::from_utf8_lossy(&body)
	);

	let json: serde_json::Value =
		serde_json::from_slice(&body).expect("Failed to parse admin search raw response.");
	let item = json["items"]
		.as_array()
		.expect("Missing items in admin search raw response.")
		.first()
		.expect("Expected at least one raw search item.");

	item["source_ref"].clone()
}

async fn contract_json() -> serde_json::Value {
	let app = routes::contract_router::<()>();
	let response = app
		.oneshot(
			Request::builder()
				.uri(OPENAPI_JSON_PATH)
				.body(Body::empty())
				.expect("Failed to build OpenAPI request."),
		)
		.await
		.expect("Failed to call OpenAPI route.");

	assert_eq!(response.status(), StatusCode::OK);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read OpenAPI response body.");

	serde_json::from_slice(&body).expect("Failed to parse OpenAPI response.")
}
