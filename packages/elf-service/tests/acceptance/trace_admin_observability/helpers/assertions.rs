use uuid::Uuid;

use crate::acceptance::trace_admin_observability::helpers::{PROJECT_ID, TENANT_ID};
use elf_service::{ElfService, SearchExplainRequest, TraceGetRequest, TraceTrajectoryGetRequest};

pub(crate) async fn assert_trace_admin_visibility_cross_scope(
	service: &ElfService,
	trace_id: Uuid,
	item_id: Uuid,
) {
	let cross_agent_trace_get = service
		.trace_get(TraceGetRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: "different_agent".to_string(),
			trace_id,
		})
		.await
		.expect("Expected cross-agent trace lookup to bypass agent ownership filtering.");

	assert_eq!(cross_agent_trace_get.trace.trace_id, trace_id);
	assert_eq!(cross_agent_trace_get.trace.agent_id, "agent_two");

	let cross_agent_trajectory = service
		.trace_trajectory_get(TraceTrajectoryGetRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: "different_agent".to_string(),
			trace_id,
		})
		.await
		.expect("Expected cross-agent trajectory lookup to bypass agent ownership filtering.");

	assert_eq!(cross_agent_trajectory.trace.trace_id, trace_id);

	let cross_agent_item = service
		.search_explain(SearchExplainRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: "different_agent".to_string(),
			result_handle: item_id,
		})
		.await
		.expect("Expected cross-agent trace-item lookup to bypass agent ownership filtering.");

	assert_eq!(cross_agent_item.item.result_handle, item_id);
}
