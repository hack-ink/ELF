use crate::docs::{DocType, Map, Value};

pub(in crate::docs::source_capture) fn source_type(
	source_ref: &Map<String, Value>,
	doc_type: DocType,
) -> String {
	source_ref
		.get("source_kind")
		.and_then(Value::as_str)
		.filter(|value| !value.trim().is_empty())
		.unwrap_or_else(|| doc_type.as_str())
		.to_string()
}

pub(in crate::docs::source_capture) fn source_origin(
	source_ref: &Map<String, Value>,
	doc_type: DocType,
) -> String {
	if let Some(origin) = source_ref_string(source_ref, "canonical_uri")
		.or_else(|| source_ref_string(source_ref, "url"))
		.or_else(|| source_ref_string(source_ref, "uri"))
	{
		return origin.to_string();
	}

	match doc_type {
		DocType::Chat => source_ref_string(source_ref, "message_id")
			.map(|message_id| {
				format!(
					"thread:{}#{}",
					source_ref_string(source_ref, "thread_id").unwrap_or("unknown"),
					message_id
				)
			})
			.unwrap_or_else(|| {
				format!(
					"thread:{}",
					source_ref_string(source_ref, "thread_id").unwrap_or("unknown")
				)
			}),
		DocType::Search => source_ref_string(source_ref, "domain")
			.map(|domain| format!("search:{domain}"))
			.unwrap_or_else(|| "search:unknown".to_string()),
		DocType::Dev => dev_origin(source_ref),
		DocType::Knowledge => source_ref_string(source_ref, "ts")
			.map(|ts| format!("knowledge:{ts}"))
			.unwrap_or_else(|| "knowledge:unknown".to_string()),
	}
}

pub(in crate::docs::source_capture) fn source_identity_value(
	source_ref: &Map<String, Value>,
	doc_type: DocType,
) -> Value {
	if let Some(canonical_uri) = source_ref_string(source_ref, "canonical_uri") {
		return serde_json::json!(["canonical_uri", canonical_uri]);
	}

	match doc_type {
		DocType::Chat => serde_json::json!([
			"chat",
			source_ref_string(source_ref, "thread_id"),
			source_ref_string(source_ref, "message_id"),
			source_ref_string(source_ref, "role"),
			source_ref_string(source_ref, "ts"),
		]),
		DocType::Search => serde_json::json!([
			"search",
			source_ref_string(source_ref, "url"),
			source_ref_string(source_ref, "domain"),
			source_ref_string(source_ref, "query"),
			source_ref_string(source_ref, "ts"),
		]),
		DocType::Dev => serde_json::json!([
			"dev",
			source_ref_string(source_ref, "repo"),
			source_ref_string(source_ref, "path"),
			source_ref_string(source_ref, "commit_sha"),
			source_ref_i64(source_ref, "pr_number"),
			source_ref_i64(source_ref, "issue_number"),
		]),
		DocType::Knowledge => serde_json::json!([
			"knowledge",
			source_ref_string(source_ref, "uri"),
			source_ref_string(source_ref, "ts"),
		]),
	}
}

fn dev_origin(source_ref: &Map<String, Value>) -> String {
	let repo = source_ref_string(source_ref, "repo").unwrap_or("unknown");
	let path = source_ref_string(source_ref, "path").unwrap_or("");
	let revision = source_ref_string(source_ref, "commit_sha")
		.map(|commit| format!("@{commit}"))
		.or_else(|| source_ref_i64(source_ref, "pr_number").map(|pr| format!("#pr-{pr}")))
		.or_else(|| {
			source_ref_i64(source_ref, "issue_number").map(|issue| format!("#issue-{issue}"))
		})
		.unwrap_or_default();

	if path.is_empty() {
		format!("repo:{repo}{revision}")
	} else {
		format!("repo:{repo}/{path}{revision}")
	}
}

fn source_ref_string<'a>(source_ref: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
	source_ref.get(key).and_then(Value::as_str).filter(|value| !value.trim().is_empty())
}

fn source_ref_i64(source_ref: &Map<String, Value>, key: &str) -> Option<i64> {
	source_ref.get(key).and_then(Value::as_i64)
}
