use crate::{
	Error,
	search::{
		Condition, Filter, MinShould, ORG_PROJECT_ID, PayloadLevel, RawSearchPath, Result,
		SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1, SearchItem, SearchTrajectoryStage,
		SearchTrajectorySummary, SearchTrajectorySummaryStage, english_gate,
	},
};

pub(super) fn apply_payload_level_to_search_item(
	mut item: SearchItem,
	payload_level: PayloadLevel,
) -> SearchItem {
	if payload_level == PayloadLevel::L2 {
		return item;
	}

	item.source_ref = serde_json::json!({});

	item
}

pub(super) fn validate_search_request_inputs(
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	query: &str,
) -> Result<()> {
	if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
		return Err(Error::InvalidRequest {
			message: "tenant_id, project_id, and agent_id are required.".to_string(),
		});
	}
	if !english_gate::is_english_natural_language(query) {
		return Err(Error::NonEnglishInput { field: "$.query".to_string() });
	}

	Ok(())
}

pub(super) fn raw_search_path_label(path: RawSearchPath) -> &'static str {
	match path {
		RawSearchPath::Quick => "quick",
		RawSearchPath::Planned => "planned",
	}
}

pub(super) fn sorted_unique_strings(mut values: Vec<String>) -> Vec<String> {
	values.sort();
	values.dedup();

	values
}

pub(super) fn build_trajectory_summary_from_stages(
	stages: &[SearchTrajectoryStage],
) -> SearchTrajectorySummary {
	let summary_stages = stages
		.iter()
		.map(|stage| {
			let stats =
				stage.stage_payload.get("stats").cloned().unwrap_or_else(|| serde_json::json!({}));

			SearchTrajectorySummaryStage {
				stage_order: stage.stage_order,
				stage_name: stage.stage_name.clone(),
				item_count: stage.items.len() as u32,
				stats,
			}
		})
		.collect();

	SearchTrajectorySummary {
		schema: SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1.to_string(),
		stages: summary_stages,
	}
}

pub(super) fn build_search_filter(
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	allowed_scopes: &[String],
) -> Filter {
	let private_scope = "agent_private".to_string();
	let non_private_scopes: Vec<String> =
		allowed_scopes.iter().filter(|scope| *scope != "agent_private").cloned().collect();
	let mut scope_should_conditions = Vec::new();

	if allowed_scopes.iter().any(|scope| scope == "agent_private") {
		let private_filter = Filter::all([
			Condition::matches("scope", private_scope),
			Condition::matches("agent_id", agent_id.to_string()),
		]);

		scope_should_conditions.push(Condition::from(private_filter));
	}
	if !non_private_scopes.is_empty() {
		scope_should_conditions.push(Condition::matches("scope", non_private_scopes));
	}

	let scope_min_should = if scope_should_conditions.is_empty() {
		None
	} else {
		Some(MinShould { min_count: 1, conditions: scope_should_conditions })
	};
	let mut project_or_org_branches = vec![Condition::from(Filter {
		must: vec![Condition::matches("project_id", project_id.to_string())],
		should: Vec::new(),
		must_not: Vec::new(),
		min_should: scope_min_should,
	})];

	if allowed_scopes.iter().any(|scope| scope == "org_shared") {
		let org_filter = Filter::all([
			Condition::matches("project_id", ORG_PROJECT_ID.to_string()),
			Condition::matches("scope", "org_shared".to_string()),
		]);

		project_or_org_branches.push(Condition::from(org_filter));
	}

	Filter {
		must: vec![
			Condition::matches("tenant_id", tenant_id.to_string()),
			Condition::matches("status", "active".to_string()),
		],
		should: Vec::new(),
		must_not: Vec::new(),
		min_should: Some(MinShould { min_count: 1, conditions: project_or_org_branches }),
	}
}
