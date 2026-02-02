use std::time::{SystemTime, UNIX_EPOCH};

fn sample_toml(reject_cjk: bool) -> String {
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
base_url = "http://localhost"
api_key = "key"
path = "/embeddings"
model = "model"
dimensions = 1536
timeout_ms = 1000
default_headers = {{}}

[providers.rerank]
provider_id = "rerank"
base_url = "http://localhost"
api_key = "key"
path = "/rerank"
model = "model"
timeout_ms = 1000
default_headers = {{}}

[providers.llm_extractor]
provider_id = "llm"
base_url = "http://localhost"
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
		reject_cjk = reject_cjk
	)
}

#[test]
fn reject_cjk_must_be_true() {
	let nanos = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("System time must be valid.")
		.as_nanos();
	let mut path = std::env::temp_dir();
	path.push(format!("elf_config_test_{nanos}.toml"));

	let payload = sample_toml(false);
	std::fs::write(&path, payload).expect("Failed to write test config.");

	let result = elf_config::load(&path);
	std::fs::remove_file(&path).expect("Failed to remove test config.");

	let err = result.expect_err("Expected reject_cjk validation error.");
	let message = err.to_string();
	assert!(
		message.contains("security.reject_cjk must be true."),
		"Unexpected error message: {message}"
	);
}
