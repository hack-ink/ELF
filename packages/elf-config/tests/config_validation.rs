use std::{
	collections::HashMap,
	env, fs,
	path::PathBuf,
	sync::atomic::{AtomicU64, Ordering},
	time::{SystemTime, UNIX_EPOCH},
};

use toml::Value;

use elf_config::{Config, Context, Error};

const SAMPLE_CONFIG_TEMPLATE_TOML: &str = include_str!("fixtures/sample_config.template.toml");

fn sample_toml(reject_cjk: bool) -> String {
	sample_toml_with_recursive(reject_cjk, false, 2, 4, 32, 256)
}

fn sample_toml_with_recursive(
	reject_cjk: bool,
	recursive_enabled: bool,
	max_depth: i64,
	max_children_per_node: i64,
	max_nodes_per_scope: i64,
	max_total_nodes: i64,
) -> String {
	let mut value: Value =
		toml::from_str(SAMPLE_CONFIG_TEMPLATE_TOML).expect("Failed to parse template config.");
	let root = value.as_table_mut().expect("Template config must be a table.");
	let search = root
		.get_mut("search")
		.and_then(Value::as_table_mut)
		.expect("Template config must include [search].");
	let recursive = search
		.get_mut("recursive")
		.and_then(Value::as_table_mut)
		.expect("Template config must include [search.recursive].");

	recursive.insert("enabled".to_string(), Value::Boolean(recursive_enabled));
	recursive.insert("max_depth".to_string(), Value::Integer(max_depth));
	recursive.insert("max_children_per_node".to_string(), Value::Integer(max_children_per_node));
	recursive.insert("max_nodes_per_scope".to_string(), Value::Integer(max_nodes_per_scope));
	recursive.insert("max_total_nodes".to_string(), Value::Integer(max_total_nodes));

	let security = root
		.get_mut("security")
		.and_then(Value::as_table_mut)
		.expect("Template config must include [security].");

	security.insert("reject_cjk".to_string(), Value::Boolean(reject_cjk));

	toml::to_string(&value).expect("Failed to render template config.")
}

fn sample_toml_with_cache(
	reject_cjk: bool,
	expansion_ttl_days: i64,
	rerank_ttl_days: i64,
	cache_enabled: bool,
) -> String {
	let mut value: Value =
		toml::from_str(&sample_toml_with_recursive(reject_cjk, false, 2, 4, 32, 256))
			.expect("Failed to parse template config.");
	let root = value.as_table_mut().expect("Template config must be a table.");
	let search = root
		.get_mut("search")
		.and_then(Value::as_table_mut)
		.expect("Template config must include [search].");
	let cache = search
		.get_mut("cache")
		.and_then(Value::as_table_mut)
		.expect("Template config must include [search.cache].");

	cache.insert("enabled".to_string(), Value::Boolean(cache_enabled));
	cache.insert("expansion_ttl_days".to_string(), Value::Integer(expansion_ttl_days));
	cache.insert("rerank_ttl_days".to_string(), Value::Integer(rerank_ttl_days));

	toml::to_string(&value).expect("Failed to render template config.")
}

fn write_temp_config(payload: String) -> PathBuf {
	static COUNTER: AtomicU64 = AtomicU64::new(0);

	let nanos = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("System time must be valid.")
		.as_nanos();
	let ordinal = COUNTER.fetch_add(1, Ordering::SeqCst);
	let pid = std::process::id();
	let mut path = env::temp_dir();

	path.push(format!("elf_config_test_{nanos}_{pid}_{ordinal}.toml"));

	fs::write(&path, payload).expect("Failed to write test config.");

	path
}

fn base_config() -> Config {
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
	let payload = sample_toml_with_cache(true, 0, 7, true);
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
fn recursive_search_settings_can_be_valid() {
	let mut cfg = base_config();

	cfg.search.recursive.enabled = true;
	cfg.search.recursive.max_depth = 4;
	cfg.search.recursive.max_children_per_node = 12;
	cfg.search.recursive.max_nodes_per_scope = 64;
	cfg.search.recursive.max_total_nodes = 120;

	assert!(elf_config::validate(&cfg).is_ok());
}

#[test]
fn recursive_search_settings_require_valid_depth_bounds() {
	let mut cfg = base_config();

	cfg.search.recursive.enabled = true;
	cfg.search.recursive.max_depth = 0;

	let err =
		elf_config::validate(&cfg).expect_err("Expected recursive max_depth validation error.");

	assert!(
		err.to_string().contains("search.recursive.max_depth must be greater than zero."),
		"Unexpected error: {err}"
	);
}

#[test]
fn recursive_search_settings_require_reasonable_bounds() {
	let mut cfg = base_config();

	cfg.search.recursive.enabled = true;
	cfg.search.recursive.max_children_per_node = 0;

	let err =
		elf_config::validate(&cfg).expect_err("Expected recursive branch factor validation error.");

	assert!(
		err.to_string()
			.contains("search.recursive.max_children_per_node must be greater than zero."),
		"Unexpected error: {err}"
	);

	cfg = base_config();
	cfg.search.recursive.enabled = true;
	cfg.search.recursive.max_total_nodes = 8;
	cfg.search.recursive.max_nodes_per_scope = 12;

	let err = elf_config::validate(&cfg)
		.expect_err("Expected recursive max_total_nodes lower-bound validation error.");

	assert!(
		err.to_string().contains(
			"search.recursive.max_total_nodes must be at least search.recursive.max_nodes_per_scope."
		),
		"Unexpected error: {err}"
	);
}

#[test]
fn graph_context_settings_max_facts_per_item_must_be_positive_when_enabled() {
	let mut cfg = base_config();

	cfg.search.graph_context.enabled = true;
	cfg.search.graph_context.max_facts_per_item = 0;

	let err = elf_config::validate(&cfg)
		.expect_err("Expected graph_context max_facts_per_item validation error.");

	assert!(
		err.to_string()
			.contains("search.graph_context.max_facts_per_item must be greater than zero."),
		"Unexpected error: {err}"
	);
}

#[test]
fn graph_context_settings_max_evidence_notes_per_fact_must_be_positive_when_enabled() {
	let mut cfg = base_config();

	cfg.search.graph_context.enabled = true;
	cfg.search.graph_context.max_evidence_notes_per_fact = 0;

	let err = elf_config::validate(&cfg)
		.expect_err("Expected graph_context max_evidence_notes_per_fact validation error.");

	assert!(
		err.to_string().contains(
			"search.graph_context.max_evidence_notes_per_fact must be greater than zero."
		),
		"Unexpected error: {err}"
	);
}

#[test]
fn graph_context_settings_max_facts_per_item_cannot_exceed_hard_limit() {
	let mut cfg = base_config();

	cfg.search.graph_context.enabled = true;
	cfg.search.graph_context.max_facts_per_item = 1_001;

	let err = elf_config::validate(&cfg)
		.expect_err("Expected graph_context max_facts_per_item upper-bound validation error.");

	assert!(
		err.to_string().contains("search.graph_context.max_facts_per_item must be 1,000 or less."),
		"Unexpected error: {err}"
	);
}

#[test]
fn graph_context_settings_max_evidence_notes_per_fact_cannot_exceed_hard_limit() {
	let mut cfg = base_config();

	cfg.search.graph_context.enabled = true;
	cfg.search.graph_context.max_evidence_notes_per_fact = 1_001;

	let err = elf_config::validate(&cfg).expect_err(
		"Expected graph_context max_evidence_notes_per_fact upper-bound validation error.",
	);

	assert!(
		err.to_string()
			.contains("search.graph_context.max_evidence_notes_per_fact must be 1,000 or less."),
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
fn chunking_tokenizer_repo_cannot_be_empty_or_whitespace() {
	let mut payload = sample_toml(true);

	payload = payload.replace("tokenizer_repo = \"REPLACE_ME\"", "tokenizer_repo = \"   \"");

	let path = write_temp_config(payload);
	let err = elf_config::load(&path).expect_err("Expected tokenizer validation error.");

	fs::remove_file(&path).expect("Failed to remove test config.");

	assert!(err.to_string().contains("chunking.tokenizer_repo must be a non-empty string."));
}

#[test]
fn chunking_tokenizer_repo_is_required() {
	let mut payload = sample_toml(true);

	payload = payload.replace("tokenizer_repo = \"REPLACE_ME\"\n", "");

	let path = write_temp_config(payload);
	let err = elf_config::load(&path).expect_err("Expected missing tokenizer_repo parse error.");

	fs::remove_file(&path).expect("Failed to remove test config.");

	let message = match err {
		Error::ParseConfig { source, .. } => source.to_string(),
		err => panic!("Expected parse config error, got {err}"),
	};

	assert!(
		message.contains("missing field `tokenizer_repo`")
			|| message.contains("missing field `tokenizer repo`"),
		"Unexpected error: {message}"
	);
}

#[test]
fn context_scope_boost_weight_requires_scope_descriptions_when_enabled() {
	let mut cfg = base_config();

	cfg.context = Some(Context {
		project_descriptions: None,
		scope_descriptions: None,
		scope_boost_weight: Some(0.1),
	});

	let err = elf_config::validate(&cfg).expect_err("Expected context validation error.");

	assert!(
		err.to_string().contains(
			"context.scope_descriptions must be non-empty when context.scope_boost_weight is greater than zero."
		),
		"Unexpected error: {err}"
	);
}

#[test]
fn context_scope_boost_weight_accepts_zero_without_descriptions() {
	let mut cfg = base_config();

	cfg.context = Some(Context {
		project_descriptions: None,
		scope_descriptions: None,
		scope_boost_weight: Some(0.0),
	});

	assert!(elf_config::validate(&cfg).is_ok());
}

#[test]
fn context_scope_boost_weight_must_be_finite() {
	let mut cfg = base_config();
	let mut scope_descriptions = HashMap::new();

	scope_descriptions.insert("project_shared".to_string(), "Project notes.".to_string());

	cfg.context = Some(Context {
		project_descriptions: None,
		scope_descriptions: Some(scope_descriptions),
		scope_boost_weight: Some(f32::NAN),
	});

	let err = elf_config::validate(&cfg).expect_err("Expected context validation error.");

	assert!(
		err.to_string().contains("context.scope_boost_weight must be a finite number."),
		"Unexpected error: {err}"
	);
}

#[test]
fn context_scope_boost_weight_must_be_in_range() {
	let mut cfg = base_config();
	let mut scope_descriptions = HashMap::new();

	scope_descriptions.insert("project_shared".to_string(), "Project notes.".to_string());

	cfg.context = Some(Context {
		project_descriptions: None,
		scope_descriptions: Some(scope_descriptions.clone()),
		scope_boost_weight: Some(-0.01),
	});

	let err = elf_config::validate(&cfg).expect_err("Expected context validation error.");

	assert!(
		err.to_string().contains("context.scope_boost_weight must be zero or greater."),
		"Unexpected error: {err}"
	);

	cfg.context = Some(Context {
		project_descriptions: None,
		scope_descriptions: Some(scope_descriptions),
		scope_boost_weight: Some(1.01),
	});

	let err = elf_config::validate(&cfg).expect_err("Expected context validation error.");

	assert!(
		err.to_string().contains("context.scope_boost_weight must be 1.0 or less."),
		"Unexpected error: {err}"
	);
}

#[test]
fn elf_example_toml_is_valid() {
	let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

	path.push("../../elf.example.toml");

	elf_config::load(&path).expect("Expected elf.example.toml to be a valid config.");
}

#[test]
fn retrieval_source_weights_must_be_non_negative() {
	let mut cfg = base_config();

	cfg.ranking.retrieval_sources.fusion_weight = -0.1;

	let err =
		elf_config::validate(&cfg).expect_err("Expected retrieval source weight validation error.");

	assert!(
		err.to_string()
			.contains("ranking.retrieval_sources.fusion_weight must be zero or greater."),
		"Unexpected error: {err}"
	);
}

#[test]
fn retrieval_source_weights_require_at_least_one_positive() {
	let mut cfg = base_config();

	cfg.ranking.retrieval_sources.fusion_weight = 0.0;
	cfg.ranking.retrieval_sources.structured_field_weight = 0.0;

	let err = elf_config::validate(&cfg)
		.expect_err("Expected retrieval source at-least-one-positive validation error.");

	assert!(
		err.to_string().contains("At least one retrieval source weight must be greater than zero."),
		"Unexpected error: {err}"
	);
}

#[test]
fn security_auth_keys_require_unique_token_ids() {
	let mut cfg = base_config();

	cfg.security.auth_mode = "static_keys".to_string();
	cfg.security.auth_keys = vec![
		elf_config::SecurityAuthKey {
			token_id: "k1".to_string(),
			token: "secret-1".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: elf_config::SecurityAuthRole::User,
		},
		elf_config::SecurityAuthKey {
			token_id: "k1".to_string(),
			token: "secret-2".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: elf_config::SecurityAuthRole::Admin,
		},
	];

	let err =
		elf_config::validate(&cfg).expect_err("Expected duplicate token_id validation error.");

	assert!(
		err.to_string().contains("token_id must be unique across security.auth_keys."),
		"Unexpected error: {err}"
	);
}

#[test]
fn security_auth_keys_require_known_read_profile() {
	let mut cfg = base_config();

	cfg.security.auth_mode = "static_keys".to_string();
	cfg.security.auth_keys = vec![elf_config::SecurityAuthKey {
		token_id: "k1".to_string(),
		token: "secret-1".to_string(),
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: Some("a".to_string()),
		read_profile: "unknown".to_string(),
		role: elf_config::SecurityAuthRole::User,
	}];

	let err =
		elf_config::validate(&cfg).expect_err("Expected auth key read_profile validation error.");

	assert!(
		err.to_string().contains(
			"read_profile must be one of private_only, private_plus_project, or all_scopes."
		),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_min_confidence_must_be_finite() {
	let mut cfg = base_config();

	cfg.memory.policy.rules.push(elf_config::MemoryPolicyRule {
		min_confidence: Some(f32::NAN),
		..Default::default()
	});

	let err = elf_config::validate(&cfg).expect_err("Expected min_confidence validation error.");

	assert!(
		err.to_string().contains("memory.policy.rules[1].min_confidence must be a finite number."),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_min_confidence_must_be_in_range() {
	let mut cfg = base_config();

	cfg.memory
		.policy
		.rules
		.push(elf_config::MemoryPolicyRule { min_confidence: Some(1.01), ..Default::default() });

	let err =
		elf_config::validate(&cfg).expect_err("Expected min_confidence range validation error.");

	assert!(
		err.to_string()
			.contains("memory.policy.rules[1].min_confidence must be between 0.0 and 1.0."),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_min_importance_must_be_finite() {
	let mut cfg = base_config();

	cfg.memory.policy.rules.push(elf_config::MemoryPolicyRule {
		min_importance: Some(f32::INFINITY),
		..Default::default()
	});

	let err = elf_config::validate(&cfg).expect_err("Expected min_importance validation error.");

	assert!(
		err.to_string().contains("memory.policy.rules[1].min_importance must be a finite number."),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_min_importance_must_be_in_range() {
	let mut cfg = base_config();

	cfg.memory
		.policy
		.rules
		.push(elf_config::MemoryPolicyRule { min_importance: Some(-0.01), ..Default::default() });

	let err =
		elf_config::validate(&cfg).expect_err("Expected min_importance range validation error.");

	assert!(
		err.to_string()
			.contains("memory.policy.rules[1].min_importance must be between 0.0 and 1.0."),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_note_type_must_be_known_value() {
	let mut cfg = base_config();

	cfg.memory.policy.rules.push(elf_config::MemoryPolicyRule {
		note_type: Some("unknown".to_string()),
		..Default::default()
	});

	let err = elf_config::validate(&cfg).expect_err("Expected note_type validation error.");

	assert!(
		err.to_string().contains(
			"memory.policy.rules[1].note_type must be one of preference, constraint, decision, profile, fact, or plan."
		),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_scope_must_be_allowed() {
	let mut cfg = base_config();

	cfg.memory.policy.rules.push(elf_config::MemoryPolicyRule {
		scope: Some("invalid_scope".to_string()),
		..Default::default()
	});

	let err = elf_config::validate(&cfg).expect_err("Expected scope validation error.");

	assert!(
		err.to_string().contains("memory.policy.rules[1].scope must be one of allowed scopes."),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_rule_pairs_must_be_unique() {
	let mut cfg = base_config();

	cfg.memory.policy.rules.push(elf_config::MemoryPolicyRule::default());
	cfg.memory.policy.rules.push(elf_config::MemoryPolicyRule::default());

	let err = elf_config::validate(&cfg).expect_err("Expected duplicate rule validation error.");

	assert!(
		err.to_string()
			.contains("memory.policy.rules[2] has a duplicate note_type and scope pair."),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_note_type_must_not_be_whitespace_only() {
	let mut cfg = base_config();

	cfg.memory.policy.rules.push(elf_config::MemoryPolicyRule {
		note_type: Some("   ".to_string()),
		..Default::default()
	});

	let err =
		elf_config::validate(&cfg).expect_err("Expected whitespace note_type validation error.");

	assert!(
		err.to_string()
			.contains("memory.policy.rules[1].note_type cannot be blank or whitespace-only."),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_scope_must_not_be_whitespace_only() {
	let mut cfg = base_config();

	cfg.memory.policy.rules.push(elf_config::MemoryPolicyRule {
		scope: Some("   ".to_string()),
		..Default::default()
	});

	let err = elf_config::validate(&cfg).expect_err("Expected whitespace scope validation error.");

	assert!(
		err.to_string()
			.contains("memory.policy.rules[1].scope cannot be blank or whitespace-only."),
		"Unexpected error: {err}"
	);
}
