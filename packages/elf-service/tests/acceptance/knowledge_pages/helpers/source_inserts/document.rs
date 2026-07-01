use time::OffsetDateTime;
use uuid::Uuid;

use crate::acceptance::knowledge_pages::helpers::{AGENT_ID, PROJECT_ID, TENANT_ID};
use elf_service::ElfService;

pub(super) async fn insert_source_document(service: &ElfService) -> (Uuid, Uuid) {
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
