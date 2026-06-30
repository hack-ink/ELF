use crate::{AGENT_ID, LoadedJob, TENANT_ID, Uuid, Value, serde_json};

pub(in crate::dreaming_readback) fn stamp_dreaming_readback_artifact(
	artifact: &mut Value,
	loaded: &LoadedJob,
	project_id: &str,
	trace_id: Uuid,
	generated_at: &str,
) {
	artifact["generated_at"] = serde_json::json!(generated_at);
	artifact["tenant_id"] = serde_json::json!(TENANT_ID);
	artifact["project_id"] = serde_json::json!(project_id);
	artifact["agent_id"] = serde_json::json!(AGENT_ID);
	artifact["read_profile"] = serde_json::json!("private_only");
	artifact["service_readback"] = serde_json::json!({
		"schema": "elf.service_native_dreaming_readback/v1",
		"job_id": loaded.job.job_id,
		"suite": loaded.job.suite,
		"runtime_path": "ElfService::list",
		"search_trace_id": trace_id,
		"source_mutation_count": 0
	});

	if loaded.job.suite == "scheduled_memory" {
		stamp_scheduled_memory_trace(artifact, trace_id);

		artifact["source_mutations"] = serde_json::json!([]);
	}
}

fn stamp_scheduled_memory_trace(artifact: &mut Value, trace_id: Uuid) {
	let trace = artifact
		.as_object_mut()
		.map(|object| object.entry("execution_trace").or_insert_with(|| serde_json::json!({})));

	if let Some(trace) = trace {
		trace["trace_id"] = serde_json::json!(format!("service-native-{trace_id}"));
		trace["trigger_kind"] = serde_json::json!("service_native_readback");
		trace["status"] = serde_json::json!("completed");
	}
}
