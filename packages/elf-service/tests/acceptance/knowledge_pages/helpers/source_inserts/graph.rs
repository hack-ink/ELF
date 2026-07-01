use time::OffsetDateTime;
use uuid::Uuid;

use crate::acceptance::knowledge_pages::helpers::{AGENT_ID, PROJECT_ID, TENANT_ID};
use elf_service::ElfService;

pub(super) async fn insert_relation(service: &ElfService, note_id: Uuid) -> Uuid {
	let subject_id = Uuid::new_v4();
	let fact_id = Uuid::new_v4();
	let evidence_id = Uuid::new_v4();

	sqlx::query(
		"\
INSERT INTO graph_entities (
	entity_id,
	tenant_id,
	project_id,
	canonical,
	canonical_norm,
	kind,
	created_at,
	updated_at
)
VALUES ($1,$2,$3,'ELF knowledge pages','elf knowledge pages','concept',$4,$4)",
	)
	.bind(subject_id)
	.bind(TENANT_ID)
	.bind(PROJECT_ID)
	.bind(OffsetDateTime::UNIX_EPOCH)
	.execute(&service.db.pool)
	.await
	.expect("graph entity should be inserted");
	sqlx::query(
		"\
INSERT INTO graph_facts (
	fact_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	subject_entity_id,
	predicate,
	predicate_id,
	object_entity_id,
	object_value,
	valid_from,
	valid_to,
	created_at,
	updated_at
)
VALUES ($1,$2,$3,$4,'project_shared',$5,'compile from',NULL,NULL,'authoritative source memory',$6,NULL,$6,$6)",
	)
	.bind(fact_id)
	.bind(TENANT_ID)
	.bind(PROJECT_ID)
	.bind(AGENT_ID)
	.bind(subject_id)
	.bind(OffsetDateTime::UNIX_EPOCH)
	.execute(&service.db.pool)
	.await
	.expect("graph fact should be inserted");
	sqlx::query(
		"\
INSERT INTO graph_fact_evidence (evidence_id, fact_id, note_id, created_at)
VALUES ($1,$2,$3,$4)",
	)
	.bind(evidence_id)
	.bind(fact_id)
	.bind(note_id)
	.bind(OffsetDateTime::UNIX_EPOCH)
	.execute(&service.db.pool)
	.await
	.expect("graph fact evidence should be inserted");

	fact_id
}
