use serde_json::Value;

pub(super) fn assert_first_generation_adapter_records(
	agentmemory: &Value,
	mem0: &Value,
	memsearch: &Value,
	claude_mem: &Value,
) {
	assert_agentmemory_first_generation_records(agentmemory);
	assert_mem0_first_generation_records(mem0);
	assert_memsearch_first_generation_records(memsearch);
	assert_claude_mem_first_generation_records(claude_mem);
}

pub(super) fn assert_agentmemory_first_generation_records(agentmemory: &Value) {
	assert_eq!(
		agentmemory.pointer("/scenarios/1/status").and_then(Value::as_str),
		Some("lifecycle_fail")
	);
	assert_eq!(
		agentmemory.pointer("/scenarios/1/elf_position").and_then(Value::as_str),
		Some("wins")
	);
	assert_eq!(agentmemory.pointer("/scenarios/2/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		agentmemory.pointer("/scenarios/2/comparison_outcome").and_then(Value::as_str),
		Some("blocked")
	);
}

pub(super) fn assert_mem0_first_generation_records(mem0: &Value) {
	assert_eq!(
		mem0.pointer("/capabilities/2/capability").and_then(Value::as_str),
		Some("local_lifecycle_update_delete_reload")
	);
	assert_eq!(mem0.pointer("/capabilities/2/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		mem0.pointer("/capabilities/3/capability").and_then(Value::as_str),
		Some("preference_correction_history")
	);
	assert_eq!(mem0.pointer("/capabilities/3/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		mem0.pointer("/capabilities/7/capability").and_then(Value::as_str),
		Some("openmemory_ui_readback")
	);
	assert_eq!(mem0.pointer("/capabilities/7/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		mem0.pointer("/capabilities/8/capability").and_then(Value::as_str),
		Some("hosted_managed_memory_claims")
	);
	assert_eq!(mem0.pointer("/capabilities/8/status").and_then(Value::as_str), Some("unsupported"));
	assert_eq!(mem0.pointer("/scenarios/0/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(mem0.pointer("/scenarios/0/elf_position").and_then(Value::as_str), Some("ties"));
	assert_eq!(
		mem0.pointer("/scenarios/1/scenario_id").and_then(Value::as_str),
		Some("preference_correction_history")
	);
	assert_eq!(mem0.pointer("/scenarios/1/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		mem0.pointer("/scenarios/1/comparison_outcome").and_then(Value::as_str),
		Some("loss")
	);
	assert_eq!(
		mem0.pointer("/scenarios/5/scenario_id").and_then(Value::as_str),
		Some("openmemory_ui_export_readback")
	);
	assert_eq!(mem0.pointer("/scenarios/5/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		mem0.pointer("/scenarios/5/command").and_then(Value::as_str),
		Some("cargo make openmemory-ui-export-readback")
	);
	assert_eq!(
		mem0.pointer("/scenarios/5/artifact").and_then(Value::as_str),
		Some("tmp/live-baseline/mem0-openmemory-ui-export.json")
	);
	assert!(
		mem0.pointer("/capabilities/7/evidence")
			.and_then(Value::as_str)
			.is_some_and(|evidence| evidence.contains("export-helper setup probe")
				&& evidence.contains("requires Docker access"))
	);
	assert_eq!(
		mem0.pointer("/scenarios/6/comparison_outcome").and_then(Value::as_str),
		Some("non_goal")
	);
}

pub(super) fn assert_memsearch_first_generation_records(memsearch: &Value) {
	assert_eq!(
		memsearch.pointer("/capabilities/2/capability").and_then(Value::as_str),
		Some("reindex_update_delete_reload")
	);
	assert_eq!(memsearch.pointer("/capabilities/2/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		memsearch.pointer("/scenarios/0/scenario_id").and_then(Value::as_str),
		Some("canonical_markdown_reindex_reload")
	);
	assert_eq!(
		memsearch.pointer("/scenarios/0/elf_position").and_then(Value::as_str),
		Some("untested")
	);
	assert_eq!(memsearch.pointer("/suites/0/status").and_then(Value::as_str), Some("not_encoded"));
	assert!(memsearch.pointer("/suites/0/evidence").and_then(Value::as_str).is_some_and(
		|evidence| evidence.contains("fixture-backed source-of-truth prompt coverage")
			&& evidence.contains("No live memsearch runtime adapter executes prompt scoring yet")
			&& evidence.contains("not a suite pass")
	));
	assert_eq!(memsearch.pointer("/suites/1/status").and_then(Value::as_str), Some("not_encoded"));
	assert!(memsearch.pointer("/suites/1/evidence").and_then(Value::as_str).is_some_and(
		|evidence| evidence.contains("fixture-backed retrieval-debug prompt coverage")
			&& evidence.contains(
				"No live memsearch runtime adapter executes retrieval prompt scoring yet"
			) && evidence.contains("not a suite pass")
	));
	assert_eq!(memsearch.pointer("/scenarios/1/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		memsearch.pointer("/scenarios/1/elf_position").and_then(Value::as_str),
		Some("untested")
	);
	assert_eq!(
		memsearch.pointer("/scenarios/3/status").and_then(Value::as_str),
		Some("unsupported")
	);
	assert_eq!(
		memsearch.pointer("/capabilities/4/capability").and_then(Value::as_str),
		Some("markdown_source_store_prompt_jobs")
	);
	assert_eq!(memsearch.pointer("/capabilities/4/status").and_then(Value::as_str), Some("pass"));
}

pub(super) fn assert_claude_mem_first_generation_records(claude_mem: &Value) {
	assert_eq!(claude_mem.pointer("/capabilities/1/status").and_then(Value::as_str), Some("real"));
	assert_eq!(
		claude_mem.pointer("/capabilities/3/capability").and_then(Value::as_str),
		Some("repository_progressive_disclosure")
	);
	assert_eq!(claude_mem.pointer("/capabilities/4/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		claude_mem.pointer("/capabilities/6/status").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(claude_mem.pointer("/suites/0/status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(claude_mem.pointer("/suites/1/status").and_then(Value::as_str), Some("blocked"));
	assert!(
		claude_mem
			.pointer("/suites/1/evidence")
			.and_then(Value::as_str)
			.is_some_and(|evidence| evidence.contains("fixture-backed progressive-disclosure")
				&& evidence.contains("viewer/operator workflow remains blocked"))
	);
	assert_eq!(claude_mem.pointer("/suites/2/status").and_then(Value::as_str), Some("blocked"));
	assert!(
		claude_mem
			.pointer("/suites/2/evidence")
			.and_then(Value::as_str)
			.is_some_and(|evidence| evidence.contains("hook capture remains blocked"))
	);
	assert_eq!(
		claude_mem.pointer("/scenarios/0/status").and_then(Value::as_str),
		Some("wrong_result")
	);
	assert_eq!(
		claude_mem.pointer("/scenarios/1/scenario_id").and_then(Value::as_str),
		Some("retrieval_repair_artifact_path")
	);
	assert_eq!(
		claude_mem.pointer("/scenarios/1/status").and_then(Value::as_str),
		Some("wrong_result")
	);
	assert!(
		claude_mem
			.pointer("/scenarios/1/evidence")
			.and_then(Value::as_str)
			.is_some_and(|evidence| evidence.contains("rerun/inspection targets")
				&& evidence.contains("tmp/live-baseline/claude-mem-checks.json"))
	);
	assert_eq!(claude_mem.pointer("/scenarios/2/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(claude_mem.pointer("/scenarios/4/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(claude_mem.pointer("/scenarios/5/status").and_then(Value::as_str), Some("blocked"));
}
