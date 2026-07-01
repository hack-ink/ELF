use std::{
	env, fs,
	path::PathBuf,
	process,
	sync::atomic::{AtomicU64, Ordering},
	time::{SystemTime, UNIX_EPOCH},
};

use toml::Value;

use elf_config::{Config, Error};

pub(crate) const TRACE_GATE_CONFIG_TOML: &str =
	include_str!("../../../../.github/fixtures/trace_gate/config.toml");
pub(crate) const SAMPLE_CONFIG_TEMPLATE_TOML: &str =
	include_str!("../fixtures/sample_config.template.toml");

pub(crate) fn sample_toml(reject_non_english: bool) -> String {
	sample_toml_with_recursive(reject_non_english, false, 2, 4, 32, 256)
}

pub(crate) fn sample_toml_with_recursive(
	reject_non_english: bool,
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

	security.insert("reject_non_english".to_string(), Value::Boolean(reject_non_english));

	toml::to_string(&value).expect("Failed to render template config.")
}

pub(crate) fn sample_toml_with_cache(
	reject_non_english: bool,
	expansion_ttl_days: i64,
	rerank_ttl_days: i64,
	cache_enabled: bool,
) -> String {
	let mut value: Value =
		toml::from_str(&sample_toml_with_recursive(reject_non_english, false, 2, 4, 32, 256))
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

pub(crate) fn write_temp_config(payload: String) -> PathBuf {
	static COUNTER: AtomicU64 = AtomicU64::new(0);

	let nanos = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("System time must be valid.")
		.as_nanos();
	let ordinal = COUNTER.fetch_add(1, Ordering::SeqCst);
	let pid = process::id();
	let mut path = env::temp_dir();

	path.push(format!("elf_config_test_{nanos}_{pid}_{ordinal}.toml"));

	fs::write(&path, payload).expect("Failed to write test config.");

	path
}

pub(crate) fn remove_required_config_key(payload: &str, path: &[&str]) -> String {
	assert!(!path.is_empty(), "Config path must not be empty.");

	let mut value: Value = toml::from_str(payload).expect("Failed to parse test config.");
	let mut table = value.as_table_mut().expect("Template config must be a table.");

	for segment in &path[..path.len() - 1] {
		table = table
			.get_mut(*segment)
			.and_then(Value::as_table_mut)
			.unwrap_or_else(|| panic!("Template config must include [{}].", segment));
	}

	let field = path[path.len() - 1];
	let removed = table.remove(field);

	assert!(removed.is_some(), "Template config must include {}.", path.join("."));

	toml::to_string(&value).expect("Failed to render template config.")
}

pub(crate) fn assert_missing_field_error(result: Result<Config, Error>, field: &str) {
	let err = result.expect_err("Expected missing required field parse error.");
	let message = match err {
		Error::ParseConfig { source, .. } => source.to_string(),
		err => panic!("Expected parse config error, got {err}"),
	};

	assert!(message.contains(&format!("missing field `{field}`")), "Unexpected error: {message}");
}

pub(crate) fn base_config() -> Config {
	let payload = sample_toml(true);

	toml::from_str(&payload).expect("Failed to parse test config.")
}
