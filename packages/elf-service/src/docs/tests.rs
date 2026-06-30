mod tests_core;
mod tests_put_validation;
mod tests_search_validation;
mod tests_source_capture;

use crate::docs::DocsSearchL0Request;

const TENANT_ID: &str = "tenant";
const PROJECT_ID: &str = "project";

fn test_request_with_query(query: &str) -> DocsSearchL0Request {
	DocsSearchL0Request {
		tenant_id: TENANT_ID.to_string(),
		project_id: PROJECT_ID.to_string(),
		caller_agent_id: "agent".to_string(),
		read_profile: "private_plus_project".to_string(),
		query: query.to_string(),
		scope: None,
		status: None,
		doc_type: None,
		sparse_mode: None,
		domain: None,
		repo: None,
		agent_id: None,
		thread_id: None,
		updated_after: None,
		updated_before: None,
		ts_gte: None,
		ts_lte: None,
		top_k: None,
		candidate_k: None,
		explain: None,
	}
}
