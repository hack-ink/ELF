use std::{
	env, fs,
	path::PathBuf,
	time::{SystemTime, UNIX_EPOCH},
};

fn sample_toml(reject_cjk: bool) -> String {
	sample_toml_with_cache(reject_cjk, 7, 7, true, "v1", "v1")
}

fn sample_toml_with_cache(
	reject_cjk: bool,
	expansion_ttl_days: i64,
	rerank_ttl_days: i64,
	cache_enabled: bool,
	expansion_version: &str,
	rerank_version: &str,
) -> String {
	format!(
		r#"[service]
http_bind = "127.0.0.1:8080"
mcp_bind = "127.0.0.1:9090"
admin_bind = "127.0.0.1:8081"
log_level = "info"

[storage.postgres]
dsn = "postgres://user:pass@127.0.0.1:5432/elf"
pool_max_conns = 5

[storage.qdrant]
url = "http://127.0.0.1:6334"
collection = "mem_notes_v1"
vector_dim = 1536

[providers.embedding]
provider_id = "embed"
api_base = "http://localhost"
api_key = "key"
path = "/embeddings"
model = "model"
dimensions = 1536
timeout_ms = 1000
default_headers = {{}}

[providers.rerank]
provider_id = "rerank"
api_base = "http://localhost"
api_key = "key"
path = "/rerank"
model = "model"
timeout_ms = 1000
default_headers = {{}}

[providers.llm_extractor]
provider_id = "llm"
api_base = "http://localhost"
api_key = "key"
path = "/chat/completions"
model = "model"
temperature = 0.1
timeout_ms = 1000
default_headers = {{}}

[scopes]
allowed = ["agent_private"]

[scopes.read_profiles]
private_only = ["agent_private"]
private_plus_project = ["agent_private"]
all_scopes = ["agent_private"]

[scopes.precedence]
agent_private = 30
project_shared = 20
org_shared = 10

[scopes.write_allowed]
agent_private = true
project_shared = true
org_shared = true

[memory]
max_notes_per_add_event = 3
max_note_chars = 240
dup_sim_threshold = 0.92
update_sim_threshold = 0.85
candidate_k = 60
top_k = 12

[chunking]
enabled = true
max_tokens = 512
overlap_tokens = 128
tokenizer_repo = ""

[search.expansion]
mode = "dynamic"
max_queries = 4
include_original = true

[search.dynamic]
min_candidates = 10
min_top_score = 0.12

[search.prefilter]
max_candidates = 0

[search.cache]
enabled = {cache_enabled}
expansion_ttl_days = {expansion_ttl_days}
rerank_ttl_days = {rerank_ttl_days}
max_payload_bytes = 262144
expansion_version = "{expansion_version}"
rerank_version = "{rerank_version}"

[search.explain]
retention_days = 7

[ranking]
recency_tau_days = 60.0
tie_breaker_weight = 0.1

[lifecycle.ttl_days]
plan = 14
fact = 180
preference = 0
constraint = 0
decision = 0
profile = 0

[lifecycle]
purge_deleted_after_days = 30
purge_deprecated_after_days = 180

[security]
bind_localhost_only = true
reject_cjk = {reject_cjk}
redact_secrets_on_write = true
evidence_min_quotes = 1
evidence_max_quotes = 2
evidence_max_quote_chars = 320
"#,
		reject_cjk = reject_cjk,
		cache_enabled = cache_enabled,
		expansion_ttl_days = expansion_ttl_days,
		rerank_ttl_days = rerank_ttl_days,
		expansion_version = expansion_version,
		rerank_version = rerank_version
	)
}

fn write_temp_config(payload: String) -> PathBuf {
	let nanos = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("System time must be valid.")
		.as_nanos();
	let mut path = env::temp_dir();
	path.push(format!("elf_config_test_{nanos}.toml"));
	fs::write(&path, payload).expect("Failed to write test config.");
	path
}

fn base_config() -> elf_config::Config {
	let payload = sample_toml(true);
	toml::from_str(&payload).expect("Failed to parse test config.")
}

#[test]
fn reject_cjk_must_be_true() {
	let payload = sample_toml(false);
	let path = write_temp_config(payload);

	let result = elf_config::load(&path);
	fs::remove_file(&path).expect("Failed to remove test config.");

	let err = result.expect_err("Expected reject_cjk validation error.");
	let message = err.to_string();
	assert!(
		message.contains("security.reject_cjk must be true."),
		"Unexpected error message: {message}"
	);
}

#[test]
fn cache_ttl_must_be_positive() {
	let payload = sample_toml_with_cache(true, 0, 7, true, "v1", "v1");
	let path = write_temp_config(payload);

	let result = elf_config::load(&path);
	fs::remove_file(&path).expect("Failed to remove test config.");

	let err = result.expect_err("Expected cache TTL validation error.");
	assert!(
		err.to_string().contains("search.cache.expansion_ttl_days must be greater than zero."),
		"Unexpected error: {err}"
	);
}

#[test]
fn chunking_config_requires_valid_bounds() {
	let mut cfg = base_config();
	cfg.chunking.max_tokens = 0;
	assert!(elf_config::validate(&cfg).is_err());

	cfg = base_config();
	cfg.chunking.overlap_tokens = cfg.chunking.max_tokens;
	assert!(elf_config::validate(&cfg).is_err());
}

#[test]
fn chunking_tokenizer_repo_can_inherit_from_embedding_model() {
	let mut cfg = base_config();
	cfg.chunking.tokenizer_repo = None;
	assert!(elf_config::validate(&cfg).is_ok());
}

#[test]
fn chunking_tokenizer_repo_empty_string_normalizes_to_none() {
	let payload = sample_toml(true);
	let path = write_temp_config(payload);

	let cfg = elf_config::load(&path).expect("Expected config to load.");
	fs::remove_file(&path).expect("Failed to remove test config.");

	assert!(cfg.chunking.tokenizer_repo.is_none());
}
