use super::*;

pub(super) fn operator_debug_output(
	adapter_kind: AdapterKind,
	loaded: &LoadedJob,
	trace_id: Option<Uuid>,
	replay_command: String,
	replay_artifact: String,
) -> (Option<serde_json::Value>, Option<OperatorDebugMaterializationEvidence>) {
	if loaded.job.suite != "operator_debugging_ux" {
		return (None, None);
	}

	let Some(source) = loaded.value.get("operator_debug") else {
		return (None, None);
	};
	let mut debug = source.clone();
	let Some(object) = debug.as_object_mut() else {
		return (None, None);
	};
	let trace_available = trace_id.is_some();
	let replay_command_available = !replay_command.trim().is_empty();
	let raw_sql_needed = false;
	let repair_action_clarity = if replay_command_available { "clear" } else { "unclear" };
	let candidate_drop_visibility =
		operator_debug_candidate_visibility(adapter_kind, object).to_string();

	object.insert("trace_available".to_string(), serde_json::Value::Bool(trace_available));
	object.insert(
		"replay_command_available".to_string(),
		serde_json::Value::Bool(replay_command_available),
	);
	object.insert("raw_sql_needed".to_string(), serde_json::Value::Bool(raw_sql_needed));
	object.insert(
		"dropped_candidate_visibility".to_string(),
		serde_json::Value::String(candidate_drop_visibility.clone()),
	);
	object.insert(
		"trace_completeness".to_string(),
		serde_json::Value::String(
			operator_debug_trace_completeness(adapter_kind, trace_available).to_string(),
		),
	);
	object.insert(
		"repair_action_clarity".to_string(),
		serde_json::Value::String(repair_action_clarity.to_string()),
	);
	object.insert("replay_command".to_string(), serde_json::Value::String(replay_command.clone()));
	object.insert("replay_artifact".to_string(), serde_json::Value::String(replay_artifact));

	match adapter_kind {
		AdapterKind::ElfServiceRuntime =>
			if let Some(trace_id) = trace_id {
				let trace_id = trace_id.to_string();

				object.insert("trace_id".to_string(), serde_json::Value::String(trace_id.clone()));
				object.insert(
					"viewer_url".to_string(),
					serde_json::Value::String(format!("/viewer?trace_id={trace_id}")),
				);
				object.insert(
					"admin_trace_bundle_url".to_string(),
					serde_json::Value::String(format!(
						"/v2/admin/traces/{trace_id}/bundle?mode=full&stage_items_limit=128&candidates_limit=200"
					)),
				);
			},
		AdapterKind::QmdCliRuntime => {
			object.remove("trace_id");
			object.remove("viewer_url");
			object.remove("admin_trace_bundle_url");
			object.insert("viewer_panels".to_string(), serde_json::json!(["qmd JSON Replay Rows"]));
		},
		AdapterKind::LightragApiContextExport => {},
	}

	let mut cli_steps = string_array_from_object(object, "cli_steps");

	push_unique(&mut cli_steps, replay_command);

	object.insert("cli_steps".to_string(), serde_json::json!(cli_steps));

	(
		Some(debug),
		Some(OperatorDebugMaterializationEvidence {
			trace_available,
			replay_command_available,
			candidate_drop_visibility,
			repair_action_clarity: repair_action_clarity.to_string(),
			raw_sql_needed,
		}),
	)
}

fn operator_debug_trace_completeness(
	adapter_kind: AdapterKind,
	trace_available: bool,
) -> &'static str {
	match adapter_kind {
		AdapterKind::ElfServiceRuntime if trace_available => "complete",
		AdapterKind::ElfServiceRuntime => "missing",
		AdapterKind::QmdCliRuntime | AdapterKind::LightragApiContextExport => "not_available",
	}
}

fn operator_debug_candidate_visibility(
	adapter_kind: AdapterKind,
	object: &Map<String, serde_json::Value>,
) -> &str {
	match adapter_kind {
		AdapterKind::ElfServiceRuntime => object
			.get("dropped_candidate_visibility")
			.and_then(serde_json::Value::as_str)
			.unwrap_or("visible through trace bundle replay candidates"),
		AdapterKind::QmdCliRuntime =>
			"qmd top-k replay output is available, but intermediate candidate-drop stages are not exposed",
		AdapterKind::LightragApiContextExport => "not encoded for this adapter",
	}
}

fn string_array_from_object(object: &Map<String, serde_json::Value>, key: &str) -> Vec<String> {
	object
		.get(key)
		.and_then(serde_json::Value::as_array)
		.map(|items| {
			items.iter().filter_map(serde_json::Value::as_str).map(ToString::to_string).collect()
		})
		.unwrap_or_default()
}

pub(super) fn elf_replay_command(trace_id: Uuid, project_id: &str) -> String {
	format!(
		"curl -fsS {} -H {} -H {} -H {}",
		shell_quote(format!(
			"http://127.0.0.1:51891/v2/admin/traces/{trace_id}/bundle?mode=full&stage_items_limit=128&candidates_limit=200"
		)
		.as_str()),
		shell_quote("X-ELF-Tenant-Id: elf-live-real-world"),
		shell_quote(format!("X-ELF-Project-Id: {project_id}").as_str()),
		shell_quote("X-ELF-Agent-Id: elf-live-real-world-agent")
	)
}

pub(super) fn qmd_replay_command(query: &str, collection: &str) -> String {
	format!(
		"npx tsx src/cli/qmd.ts query {} -c {} --json --no-rerank --min-score 0 -n 5",
		shell_quote(format!("lex: {query}\nvec: {query}").as_str()),
		shell_quote(collection)
	)
}

fn shell_quote(value: &str) -> String {
	format!("'{}'", value.replace('\'', "'\\''"))
}
