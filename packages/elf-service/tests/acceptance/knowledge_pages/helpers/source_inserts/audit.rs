use time::OffsetDateTime;
use uuid::Uuid;

use crate::acceptance::knowledge_pages::helpers::{AGENT_ID, PROJECT_ID, TENANT_ID};
use elf_service::ElfService;

pub(super) async fn insert_event_audit(service: &ElfService, note_id: Uuid) -> Uuid {
	let decision_id = Uuid::new_v4();

	sqlx::query(
		"\
INSERT INTO memory_ingest_decisions (
	decision_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	pipeline,
	note_type,
	note_key,
	note_id,
	base_decision,
	policy_decision,
	note_op,
	reason_code,
	details,
	ts
)
VALUES ($1,$2,$3,$4,'agent_private','add_event','fact','knowledge_event',$5,'remember','remember','ADD',NULL,$6,$7)",
	)
	.bind(decision_id)
	.bind(TENANT_ID)
	.bind(PROJECT_ID)
	.bind(AGENT_ID)
	.bind(note_id)
	.bind(serde_json::json!({ "fixture": "knowledge_page_event_audit" }))
	.bind(OffsetDateTime::UNIX_EPOCH)
	.execute(&service.db.pool)
	.await
	.expect("event audit should be inserted");

	decision_id
}
