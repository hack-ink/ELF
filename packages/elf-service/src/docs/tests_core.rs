use ahash::AHashMap;
use tokenizers::{Tokenizer, models::wordlevel::WordLevel, pre_tokenizers::whitespace::Whitespace};

use crate::docs::{self, DocType};

fn test_tokenizer() -> Tokenizer {
	let mut vocab = AHashMap::new();

	vocab.insert("alpha".to_string(), 1_u32);
	vocab.insert("beta".to_string(), 2_u32);
	vocab.insert("charlie".to_string(), 3_u32);
	vocab.insert("delta".to_string(), 4_u32);
	vocab.insert("<unk>".to_string(), 0_u32);

	let model = WordLevel::builder()
		.vocab(vocab)
		.unk_token("<unk>".to_string())
		.build()
		.expect("Failed to build test tokenizer.");
	let mut tokenizer = Tokenizer::new(model);

	tokenizer.with_pre_tokenizer(Some(Whitespace));

	tokenizer
}

#[test]
fn doc_type_parses_and_serializes() {
	let encoded =
		serde_json::to_string(&DocType::Knowledge).expect("Expected DocType serialization.");
	let parsed =
		serde_json::from_str::<DocType>("\"knowledge\"").expect("Expected parse to succeed.");
	let invalid: Result<DocType, _> = serde_json::from_str("\"invalid\"");

	assert_eq!(encoded, "\"knowledge\"");
	assert_eq!(parsed, DocType::Knowledge);
	assert!(invalid.is_err());
}

#[test]
fn resolve_doc_chunking_profile_is_deterministic_by_doc_type() {
	let small = docs::resolve_doc_chunking_profile(DocType::Chat);

	assert_eq!(small.max_tokens, 1_024);
	assert_eq!(small.overlap_tokens, 128);

	let default = docs::resolve_doc_chunking_profile(DocType::Knowledge);

	assert_eq!(default.max_tokens, 2_048);
	assert_eq!(default.overlap_tokens, 256);
}

#[test]
fn excerpt_level_max_supports_l0_and_rejects_unknown_level() {
	assert_eq!(
		docs::excerpt_level_max("L0").expect("Expected L0 to be supported."),
		docs::DEFAULT_L0_MAX_BYTES
	);
	assert!(docs::excerpt_level_max("L3").is_err());
}

#[test]
fn split_tokens_by_offsets_preserves_original_substring_offsets() {
	let tokenizer = test_tokenizer();
	let chunks = docs::split_tokens_by_offsets("alpha bravo charlie delta", 2, 1, 10, &tokenizer)
		.expect("Expected token chunking to succeed.");

	assert_eq!(chunks.len(), 3);
	assert_eq!(chunks[0].start_offset, 0);
	assert_eq!(chunks[0].end_offset, 11);
	assert_eq!(chunks[1].start_offset, 6);
	assert_eq!(chunks[1].end_offset, 19);
	assert_eq!(chunks[2].start_offset, 12);
	assert_eq!(chunks[2].end_offset, 25);

	for chunk in &chunks {
		assert_eq!(chunk.text, "alpha bravo charlie delta"[chunk.start_offset..chunk.end_offset]);
	}
}
