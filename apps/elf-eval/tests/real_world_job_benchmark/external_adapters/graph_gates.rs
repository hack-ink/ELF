use serde_json::Value;

pub(super) fn assert_graph_rag_research_gate_records(
	ragflow: &Value,
	lightrag: &Value,
	graphrag: &Value,
) {
	assert_eq!(ragflow.pointer("/evidence_class").and_then(Value::as_str), Some("research_gate"));
	assert_eq!(ragflow.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		ragflow.pointer("/execution_metadata/research_depth").and_then(Value::as_str),
		Some(
			"D2 feasibility verdict plus XY-885 evidence-smoke implementation and XY-900 scored smoke promotion; checked-in record remains research_gate unless a generated artifact reaches query output"
		)
	);
	assert_eq!(
		ragflow.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make smoke-ragflow-docker")
	);
	assert_eq!(
		ragflow.pointer("/result/artifact").and_then(Value::as_str),
		Some("tmp/real-world-memory/ragflow-smoke/ragflow-report.json")
	);
	assert_eq!(
		ragflow.pointer("/execution_metadata/sources/0/url").and_then(Value::as_str),
		Some("https://github.com/infiniflow/ragflow")
	);
	assert_eq!(lightrag.pointer("/evidence_class").and_then(Value::as_str), Some("research_gate"));
	assert_eq!(lightrag.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		lightrag.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make smoke-lightrag-docker-context")
	);
	assert_eq!(
		lightrag.pointer("/run/command").and_then(Value::as_str),
		Some("ELF_LIGHTRAG_CONTEXT_START=1 cargo make smoke-lightrag-docker-context")
	);
	assert_eq!(
		lightrag.pointer("/capabilities/3/status").and_then(Value::as_str),
		Some("not_encoded")
	);
	assert_eq!(graphrag.pointer("/evidence_class").and_then(Value::as_str), Some("research_gate"));
	assert_eq!(
		graphrag.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make smoke-graphrag-docker")
	);
	assert_eq!(graphrag.pointer("/suites/1/status").and_then(Value::as_str), Some("not_encoded"));
}
pub(super) fn assert_graphiti_zep_adapter(adapter: &Value) {
	assert_eq!(adapter.pointer("/evidence_class").and_then(Value::as_str), Some("research_gate"));
	assert_eq!(adapter.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		adapter.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make smoke-graphiti-zep-docker-temporal")
	);
	assert_eq!(
		adapter.pointer("/run/command").and_then(Value::as_str),
		Some(
			"ELF_GRAPHITI_ZEP_SMOKE_START=1 ELF_GRAPHITI_ZEP_SMOKE_RUN=1 cargo make smoke-graphiti-zep-docker-temporal"
		)
	);
	assert_eq!(
		adapter.pointer("/suites/0/suite_id").and_then(Value::as_str),
		Some("memory_evolution")
	);
	assert_eq!(adapter.pointer("/suites/0/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		adapter.pointer("/execution_metadata/research_depth").and_then(Value::as_str),
		Some(
			"D2 feasibility plus XY-888 Docker temporal smoke implementation and XY-900 scored smoke promotion; checked-in record remains research_gate unless a generated artifact reaches Graphiti search output"
		)
	);
}
