use crate::helpers;

#[test]
fn retrieval_source_weights_must_be_non_negative() {
	let mut cfg = helpers::base_config();

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
	let mut cfg = helpers::base_config();

	cfg.ranking.retrieval_sources.fusion_weight = 0.0;
	cfg.ranking.retrieval_sources.structured_field_weight = 0.0;

	let err = elf_config::validate(&cfg)
		.expect_err("Expected retrieval source at-least-one-positive validation error.");

	assert!(
		err.to_string().contains("At least one retrieval source weight must be greater than zero."),
		"Unexpected error: {err}"
	);
}
