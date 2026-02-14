use std::{
	collections::HashMap,
	env, fs,
	path::PathBuf,
	sync::atomic::{AtomicU64, Ordering},
	time::{SystemTime, UNIX_EPOCH},
};

use toml::Value;

use elf_config::{Config, Context};

const SAMPLE_CONFIG_TEMPLATE_TOML: &str = include_str!("fixtures/sample_config.template.toml");

fn sample_toml(reject_cjk: bool) -> String {
	sample_toml_with_cache(reject_cjk, 7, 7, true)
}

fn sample_toml_with_cache(
	reject_cjk: bool,
	expansion_ttl_days: i64,
	rerank_ttl_days: i64,
	cache_enabled: bool,
) -> String {
	let mut value: Value =
		toml::from_str(SAMPLE_CONFIG_TEMPLATE_TOML).expect("Failed to parse template config.");
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

	let security = root
		.get_mut("security")
		.and_then(Value::as_table_mut)
		.expect("Template config must include [security].");

	security.insert("reject_cjk".to_string(), Value::Boolean(reject_cjk));

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
