use super::{promotion, types};

fn payload_with_scope(scope: Option<&str>) -> types::PromotedMemoryPayload {
	types::PromotedMemoryPayload {
		note_type: "fact".to_string(),
		text: "Fact: Reviewed memory promotion is explicit.".to_string(),
		scope: scope.map(str::to_string),
		key: None,
		importance: None,
		confidence: None,
		ttl_days: None,
		source_ref: serde_json::json!({}),
	}
}

#[test]
fn promoted_memory_scope_uses_default_and_rejects_blank_override() {
	let defaulted = promotion::promoted_memory_scope(&payload_with_scope(None), "project_shared")
		.expect("missing scope should use target default");

	assert_eq!(defaulted, "project_shared");
	assert!(
		promotion::promoted_memory_scope(&payload_with_scope(Some(" ")), "agent_private").is_err()
	);
}

#[test]
fn promoted_memory_project_id_normalizes_org_shared_scope() {
	assert_eq!(
		promotion::promoted_memory_project_id("source-project", "project_shared"),
		"source-project"
	);
	assert_eq!(
		promotion::promoted_memory_project_id("source-project", "org_shared"),
		crate::access::ORG_PROJECT_ID
	);
}
