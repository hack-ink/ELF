use serde_json::Value;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Result, models::DocDocument};

/// Normalizes absent document source metadata to an empty JSON object.
pub fn normalize_source_ref(source_ref: Option<Value>) -> Value {
	source_ref.unwrap_or(Value::Object(Default::default()))
}

/// Inserts one document record into storage.
pub async fn insert_doc_document<'e, E>(executor: E, doc: &DocDocument) -> Result<()>
where
	E: PgExecutor<'e>,
{
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
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
ON CONFLICT (doc_id) DO UPDATE
SET
	tenant_id = EXCLUDED.tenant_id,
	project_id = EXCLUDED.project_id,
	agent_id = EXCLUDED.agent_id,
	scope = EXCLUDED.scope,
	doc_type = EXCLUDED.doc_type,
	status = EXCLUDED.status,
	title = EXCLUDED.title,
	source_ref = EXCLUDED.source_ref,
	content = EXCLUDED.content,
	content_bytes = EXCLUDED.content_bytes,
	content_hash = EXCLUDED.content_hash,
	updated_at = EXCLUDED.updated_at",
	)
	.bind(doc.doc_id)
	.bind(doc.tenant_id.as_str())
	.bind(doc.project_id.as_str())
	.bind(doc.agent_id.as_str())
	.bind(doc.scope.as_str())
	.bind(doc.doc_type.as_str())
	.bind(doc.status.as_str())
	.bind(doc.title.as_deref())
	.bind(&doc.source_ref)
	.bind(doc.content.as_str())
	.bind(doc.content_bytes)
	.bind(doc.content_hash.as_str())
	.bind(doc.created_at)
	.bind(doc.updated_at)
	.execute(executor)
	.await?;

	Ok(())
}

/// Fetches one document record by tenant and document identifier.
pub async fn get_doc_document<'e, E>(
	executor: E,
	tenant_id: &str,
	doc_id: Uuid,
) -> Result<Option<DocDocument>>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, DocDocument>(
		"\
	SELECT
		doc_id,
		tenant_id,
		project_id,
		agent_id,
		scope,
		doc_type,
		status,
		title,
		COALESCE(source_ref, '{}'::jsonb) AS source_ref,
		content,
		content_bytes,
		content_hash,
		created_at,
		updated_at
FROM doc_documents
WHERE tenant_id = $1 AND doc_id = $2
LIMIT 1",
	)
	.bind(tenant_id)
	.bind(doc_id)
	.fetch_optional(executor)
	.await?;

	Ok(row)
}

/// Marks one document record as deleted.
pub async fn mark_doc_deleted<'e, E>(
	executor: E,
	tenant_id: &str,
	doc_id: Uuid,
	now: OffsetDateTime,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
UPDATE doc_documents
SET status = 'deleted', updated_at = $1
WHERE tenant_id = $2 AND doc_id = $3",
	)
	.bind(now)
	.bind(tenant_id)
	.bind(doc_id)
	.execute(executor)
	.await?;

	Ok(())
}
