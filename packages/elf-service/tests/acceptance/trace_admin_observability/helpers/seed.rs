use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::acceptance::trace_admin_observability::helpers::{
	PROJECT_ID, TENANT_ID, VisibilityTraceFixtureIds, inserts,
};
use elf_service::{ElfService, TraceRecentListRequest, TraceRecentListResponse};

pub(crate) async fn seed_visibility_and_recent_list_traces(
	service: &ElfService,
	now: OffsetDateTime,
) -> VisibilityTraceFixtureIds {
	let trace_one = Uuid::new_v4();
	let trace_two = Uuid::new_v4();
	let trace_three = Uuid::new_v4();
	let item_one = Uuid::new_v4();
	let item_two = Uuid::new_v4();
	let item_three = Uuid::new_v4();
	let note_one = Uuid::new_v4();
	let note_two = Uuid::new_v4();
	let note_three = Uuid::new_v4();
	let chunk_one = Uuid::new_v4();
	let chunk_two = Uuid::new_v4();
	let chunk_three = Uuid::new_v4();

	inserts::insert_trace(&service.db.pool, trace_one, "agent_one", "private_only", "one", now)
		.await;
	inserts::insert_trace(
		&service.db.pool,
		trace_two,
		"agent_two",
		"private_only",
		"two",
		now - Duration::seconds(10),
	)
	.await;
	inserts::insert_trace(
		&service.db.pool,
		trace_three,
		"agent_three",
		"private_only",
		"three",
		now - Duration::seconds(20),
	)
	.await;
	inserts::insert_trace_item(&service.db.pool, item_one, trace_one, note_one, chunk_one, 1).await;
	inserts::insert_trace_item(&service.db.pool, item_two, trace_two, note_two, chunk_two, 1).await;
	inserts::insert_trace_item(
		&service.db.pool,
		item_three,
		trace_three,
		note_three,
		chunk_three,
		1,
	)
	.await;

	VisibilityTraceFixtureIds { trace_one, trace_two, trace_three, item_two }
}

pub(crate) async fn trace_recent_list_page(
	service: &ElfService,
	cursor_created_at: Option<OffsetDateTime>,
	cursor_trace_id: Option<Uuid>,
) -> TraceRecentListResponse {
	service
		.trace_recent_list(TraceRecentListRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: "admin_agent".to_string(),
			limit: Some(2),
			cursor_created_at,
			cursor_trace_id,
			agent_id_filter: None,
			read_profile: None,
			created_after: None,
			created_before: None,
		})
		.await
		.expect("Failed to list recent traces.")
}
