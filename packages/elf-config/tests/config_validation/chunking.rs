use std::fs;

use crate::helpers;
use elf_config::Error;

#[test]
fn chunking_config_requires_valid_bounds() {
	let mut cfg = helpers::base_config();

	cfg.chunking.max_tokens = 0;

	assert!(elf_config::validate(&cfg).is_err());

	cfg = helpers::base_config();
	cfg.chunking.overlap_tokens = cfg.chunking.max_tokens;

	assert!(elf_config::validate(&cfg).is_err());
}

#[test]
fn chunking_tokenizer_repo_cannot_be_empty_or_whitespace() {
	let mut payload = helpers::sample_toml(true);

	payload = payload.replace("tokenizer_repo = \"REPLACE_ME\"", "tokenizer_repo = \"   \"");

	let path = helpers::write_temp_config(payload);
	let err = elf_config::load(&path).expect_err("Expected tokenizer validation error.");

	fs::remove_file(&path).expect("Failed to remove test config.");

	assert!(err.to_string().contains("chunking.tokenizer_repo must be a non-empty string."));
}

#[test]
fn chunking_tokenizer_repo_is_required() {
	let mut payload = helpers::sample_toml(true);

	payload = payload.replace("tokenizer_repo = \"REPLACE_ME\"\n", "");

	let path = helpers::write_temp_config(payload);
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
