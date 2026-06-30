use crate::rerank::{local, response};

#[test]
fn aligns_scores_by_index() {
	let json = serde_json::json!({
		"results": [
			{ "index": 1, "relevance_score": 0.2 },
			{ "index": 0, "relevance_score": 0.9 }
		]
	});
	let scores = response::parse_rerank_response(json, 2)
		.expect("Rerank response parsing must succeed for the valid JSON fixture.");

	assert_eq!(scores, vec![0.9, 0.2]);
}

#[test]
fn local_rerank_scores_match_token_overlap_fraction() {
	let scores = local::local_rerank("alpha beta", &[String::from("alpha"), String::from("gamma")]);

	assert_eq!(scores.len(), 2);
	assert!((scores[0] - 0.5).abs() < 1e-6, "Unexpected score: {}", scores[0]);
	assert_eq!(scores[1], 0.0);
}

#[test]
fn local_noisy_model_is_detected_and_nonnegative() {
	assert_eq!(local::parse_local_noisy_model("local-token-overlap"), None);
	assert_eq!(local::parse_local_noisy_model("local-token-overlap-noisy@0.02"), Some(0.02));
	assert_eq!(local::parse_local_noisy_model("local-token-overlap-noisy@-1"), Some(0.0));
}

#[test]
fn local_rerank_noisy_varies_across_calls() {
	// Use a base score away from 0 and 1 so clamping does not mask noise.
	let docs = [String::from("alpha"), String::from("alpha")];
	let first = local::local_rerank_dispatch("local-token-overlap-noisy@0.1", "alpha beta", &docs);

	assert!(first.iter().all(|v| (0.0..=1.0).contains(v)));

	let mut varied = false;

	for _ in 0..32 {
		let next =
			local::local_rerank_dispatch("local-token-overlap-noisy@0.1", "alpha beta", &docs);

		assert_eq!(first.len(), next.len());
		assert!(next.iter().all(|v| (0.0..=1.0).contains(v)));

		if next != first {
			varied = true;

			break;
		}
	}

	assert!(varied, "Expected noisy rerank to vary across calls.");
}
