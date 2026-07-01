use time::OffsetDateTime;
use uuid::Uuid;

use crate::acceptance::knowledge_pages::helpers::{
	AGENT_ID, KnowledgeSourceIds, PROJECT_ID, TENANT_ID,
};
use elf_service::{AddNoteInput, AddNoteRequest, ElfService};

pub(crate) async fn insert_rebuild_sources(service: &ElfService) -> KnowledgeSourceIds {
	let note_id = insert_source_note(
		service,
		"knowledge_pages_foundation",
		"Fact: Derived knowledge pages are rebuilt from authoritative source memory and keep citations.",
	)
	.await;
	let event_id = insert_event_audit(service, note_id).await;
	let (doc_id, chunk_id) = insert_source_document(service).await;
	let fact_id = insert_relation(service, note_id).await;
	let proposal_id = insert_applied_proposal(service, note_id).await;

	KnowledgeSourceIds { note_id, event_id, doc_id, chunk_id, fact_id, proposal_id }
}

async fn insert_source_note(service: &ElfService, key: &str, text: &str) -> Uuid {
	let response = service
		.add_note(AddNoteRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: AGENT_ID.to_string(),
			scope: "agent_private".to_string(),
			notes: vec![AddNoteInput {
				r#type: "fact".to_string(),
				key: Some(key.to_string()),
				text: text.to_string(),
				structured: None,
				importance: 0.7,
				confidence: 0.9,
				ttl_days: None,
				source_ref: serde_json::json!({ "schema": "acceptance/v1", "key": key }),
				write_policy: None,
			}],
		})
		.await
		.expect("add_note should persist source note");

	response.results[0].note_id.expect("source note id should be present")
}

async fn insert_event_audit(service: &ElfService, note_id: Uuid) -> Uuid {
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

async fn insert_source_document(service: &ElfService) -> (Uuid, Uuid) {
	let doc_id = Uuid::new_v4();
	let chunk_id = Uuid::new_v4();
	let content = "The Knowledge Workspace compiles Source Library spans into cited derived pages.";
	let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
	let chunk_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
	let source_ref = serde_json::json!({
		"schema": "doc_source_ref/v1",
		"doc_type": "knowledge",
		"uri": "docs://knowledge/workspace/source-span-fixture",
		"source_record_id": doc_id,
		"content_hash": content_hash,
		"source_spans": [
			{
				"schema": "doc_source_span/v1",
				"span_id": Uuid::new_v4(),
				"chunk_id": chunk_id,
				"status": "captured",
				"start_offset": 0,
				"end_offset": content.len(),
				"content_hash": content_hash,
				"chunk_hash": chunk_hash
			}
		]
	});

	sqlx::query(
		"\
INSERT INTO doc_documents (
	doc_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	doc_type,
	status,
	title,
	source_ref,
	content,
	content_bytes,
	content_hash,
	created_at,
	updated_at
)
VALUES ($1,$2,$3,$4,'project_shared','knowledge','active','Knowledge Workspace Source',$5,$6,$7,$8,$9,$9)",
	)
	.bind(doc_id)
	.bind(TENANT_ID)
	.bind(PROJECT_ID)
	.bind(AGENT_ID)
	.bind(source_ref)
	.bind(content)
	.bind(i32::try_from(content.len()).expect("fixture content length should fit i32"))
	.bind(content_hash)
	.bind(OffsetDateTime::UNIX_EPOCH)
	.execute(&service.db.pool)
	.await
	.expect("source document should be inserted");
	sqlx::query(
		"\
INSERT INTO doc_chunks (
	chunk_id,
	doc_id,
	chunk_index,
	start_offset,
	end_offset,
	chunk_text,
	chunk_hash,
	created_at
)
VALUES ($1,$2,0,0,$3,$4,$5,$6)",
	)
	.bind(chunk_id)
	.bind(doc_id)
	.bind(i32::try_from(content.len()).expect("fixture content length should fit i32"))
	.bind(content)
	.bind(chunk_hash)
	.bind(OffsetDateTime::UNIX_EPOCH)
	.execute(&service.db.pool)
	.await
	.expect("source document chunk should be inserted");

	(doc_id, chunk_id)
}

async fn insert_relation(service: &ElfService, note_id: Uuid) -> Uuid {
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

async fn insert_applied_proposal(service: &ElfService, note_id: Uuid) -> Uuid {
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
