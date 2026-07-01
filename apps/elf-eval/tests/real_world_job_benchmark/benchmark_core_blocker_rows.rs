use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(crate) fn assert_tracked_external_blocker_row(
	row: &Value,
	product_name: &str,
	same_corpus: bool,
) -> Result<()> {
	assert_eq!(row.pointer("/product_name").and_then(Value::as_str), Some(product_name));
	assert_eq!(row.pointer("/result_state").and_then(Value::as_str), Some("blocked"));
	assert_eq!(row.pointer("/evidence_class").and_then(Value::as_str), Some("research_gate"));
	assert_eq!(row.pointer("/comparable").and_then(Value::as_bool), Some(false));
	assert_eq!(row.pointer("/same_corpus").and_then(Value::as_bool), Some(same_corpus));
	assert_eq!(row.pointer("/source_id_mapped").and_then(Value::as_bool), Some(false));
	assert_eq!(row.pointer("/held_out").and_then(Value::as_bool), Some(false));
	assert_eq!(row.pointer("/leakage_audited").and_then(Value::as_bool), Some(false));
	assert_eq!(row.pointer("/product_runtime").and_then(Value::as_bool), Some(false));
	assert_eq!(row.pointer("/container_digest_identified").and_then(Value::as_bool), Some(false));
	assert!(row.pointer("/metrics/retrieval/recall_at_k").is_some_and(Value::is_null));
	assert!(row.pointer("/metrics/retrieval/precision_at_k").is_some_and(Value::is_null));
	assert!(row.pointer("/metrics/retrieval/mrr").is_some_and(Value::is_null));
	assert!(row.pointer("/metrics/retrieval/ndcg").is_some_and(Value::is_null));
	assert!(support::array_contains_str(
		row,
		"/next_evidence",
		"Map returned evidence to stable source ids."
	)?);
	assert!(support::array_contains_str(
		row,
		"/next_evidence",
		"Run a Docker-contained product-runtime adapter for this row."
	)?);
	assert!(support::array_contains_str(
		row,
		"/next_evidence",
		"Record container image digest evidence."
	)?);

	if same_corpus {
		assert!(!support::array_contains_str(
			row,
			"/next_evidence",
			"Map this product to the same corpus."
		)?);
	} else {
		assert!(support::array_contains_str(
			row,
			"/next_evidence",
			"Map this product to the same corpus."
		)?);
	}

	Ok(())
}
