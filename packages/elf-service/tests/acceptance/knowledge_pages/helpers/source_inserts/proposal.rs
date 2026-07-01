use time::OffsetDateTime;
use uuid::Uuid;

use crate::acceptance::knowledge_pages::helpers::{AGENT_ID, PROJECT_ID, TENANT_ID};
use elf_service::ElfService;

pub(super) async fn insert_applied_proposal(service: &ElfService, note_id: Uuid) -> Uuid {
	let run_id = Uuid::new_v4();
	let proposal_id = Uuid::new_v4();
	let source_refs = serde_json::json!([
		{
			"kind": "note",
			"id": note_id,
			"snapshot": {
				"status": "active",
				"updated_at": "1970-01-01T00:00:00Z",
				"metadata": { "fixture": "knowledge_pages" },
				"source_ref": {}
			}
		}
	]);
	let lineage = serde_json::json!({ "source_refs": source_refs });

	sqlx::query(
		"\
INSERT INTO consolidation_runs (
	run_id,
	tenant_id,
	project_id,
	agent_id,
	contract_schema,
	job_kind,
	status,
	input_refs,
	source_snapshot,
	lineage,
	error,
	created_at,
	updated_at,
	completed_at
)
VALUES ($1,$2,$3,$4,'elf.consolidation/v1','manual','completed',$5,$6,$7,'{}'::jsonb,$8,$8,$8)",
	)
	.bind(run_id)
	.bind(TENANT_ID)
	.bind(PROJECT_ID)
	.bind(AGENT_ID)
	.bind(&source_refs)
	.bind(serde_json::json!({ "source_count": 1 }))
	.bind(&lineage)
	.bind(OffsetDateTime::UNIX_EPOCH)
	.execute(&service.db.pool)
	.await
	.expect("consolidation run should be inserted");
	sqlx::query(
		"\
INSERT INTO consolidation_proposals (
	proposal_id,
	run_id,
	tenant_id,
	project_id,
	agent_id,
	contract_schema,
	proposal_kind,
	apply_intent,
	review_state,
	source_refs,
	source_snapshot,
	lineage,
	diff,
	confidence,
	unsupported_claim_flags,
	contradiction_markers,
	staleness_markers,
	target_ref,
	proposed_payload,
	reviewer_agent_id,
	review_comment,
	reviewed_at,
	created_at,
	updated_at
)
VALUES ($1,$2,$3,$4,$5,'elf.consolidation/v1','knowledge_page','create_derived_knowledge_page','applied',$6,$7,$8,$9,0.9,'[]'::jsonb,'[]'::jsonb,'[]'::jsonb,'{}'::jsonb,$10,$5,'Apply derived page proposal.',$11,$11,$11)",
	)
	.bind(proposal_id)
	.bind(run_id)
	.bind(TENANT_ID)
	.bind(PROJECT_ID)
	.bind(AGENT_ID)
	.bind(&source_refs)
	.bind(serde_json::json!({ "source_count": 1 }))
	.bind(&lineage)
	.bind(serde_json::json!({
		"summary": "Create a derived knowledge page from cited source memory.",
		"before": {},
		"after": { "page_key": "knowledge-foundation" }
	}))
	.bind(serde_json::json!({ "page_key": "knowledge-foundation" }))
	.bind(OffsetDateTime::UNIX_EPOCH)
	.execute(&service.db.pool)
	.await
	.expect("consolidation proposal should be inserted");

	proposal_id
}
