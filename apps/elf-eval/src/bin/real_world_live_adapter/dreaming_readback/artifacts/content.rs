use crate::Value;

pub(in crate::dreaming_readback) fn dreaming_readback_content(
	suite: &str,
	artifacts: &[Value],
) -> String {
	let mut parts = Vec::new();

	for artifact in artifacts {
		match suite {
			"memory_summary" => push_memory_summary_text(artifact, &mut parts),
			"proactive_brief" => push_proactive_brief_text(artifact, &mut parts),
			"scheduled_memory" => push_scheduled_memory_text(artifact, &mut parts),
			_ => {},
		}
	}

	if parts.is_empty() {
		"Service-native Dreaming readback produced no artifact text.".to_string()
	} else {
		parts.join(" ")
	}
}

fn push_memory_summary_text(artifact: &Value, parts: &mut Vec<String>) {
	for entry in artifact.get("entries").and_then(Value::as_array).into_iter().flatten() {
		if let Some(text) = entry.get("text").and_then(Value::as_str) {
			parts.push(text.to_string());
		}
	}
}

fn push_proactive_brief_text(artifact: &Value, parts: &mut Vec<String>) {
	for suggestion in artifact.get("suggestions").and_then(Value::as_array).into_iter().flatten() {
		if let Some(title) = suggestion.get("title").and_then(Value::as_str) {
			parts.push(title.to_string());
		}
		if let Some(body) = suggestion.get("body").and_then(Value::as_str) {
			parts.push(body.to_string());
		}
	}
}

fn push_scheduled_memory_text(artifact: &Value, parts: &mut Vec<String>) {
	for output in artifact.get("outputs").and_then(Value::as_array).into_iter().flatten() {
		if let Some(text) = output.get("text").and_then(Value::as_str) {
			parts.push(text.to_string());
		}
	}
}
