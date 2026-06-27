//! Work Journal persistence queries.

use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{Error, Result, models::WorkJournalEntry};

/// Inserts one Work Journal entry row.
pub async fn insert_work_journal_entry<'e, E>(executor: E, entry: &WorkJournalEntry) -> Result<()>
where
	E: PgExecutor<'e>,
{
	let result = sqlx::query(
		"\
INSERT INTO work_journal_entries (
	entry_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	session_id,
	family,
	status,
	title,
	body,
	source_refs,
	explicit_next_steps,
	inferred_next_steps,
	rejected_options,
	promotion_boundary,
	redaction_audit,
	created_at,
	updated_at
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)
ON CONFLICT (entry_id) DO NOTHING",
	)
	.bind(entry.entry_id)
	.bind(entry.tenant_id.as_str())
	.bind(entry.project_id.as_str())
	.bind(entry.agent_id.as_str())
	.bind(entry.scope.as_str())
	.bind(entry.session_id.as_str())
	.bind(entry.family.as_str())
	.bind(entry.status.as_str())
	.bind(entry.title.as_deref())
	.bind(entry.body.as_str())
	.bind(&entry.source_refs)
	.bind(&entry.explicit_next_steps)
	.bind(&entry.inferred_next_steps)
	.bind(&entry.rejected_options)
	.bind(&entry.promotion_boundary)
	.bind(&entry.redaction_audit)
	.bind(entry.created_at)
	.bind(entry.updated_at)
	.execute(executor)
	.await?;

	if result.rows_affected() == 0 {
		return Err(Error::Conflict("work_journal entry_id already exists".to_string()));
	}

	Ok(())
}

/// Fetches one Work Journal entry by tenant and entry identifier.
pub async fn get_work_journal_entry<'e, E>(
	executor: E,
	tenant_id: &str,
	entry_id: Uuid,
) -> Result<Option<WorkJournalEntry>>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, WorkJournalEntry>(
		"\
SELECT
	entry_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	session_id,
	family,
	status,
	title,
	body,
	COALESCE(source_refs, '[]'::jsonb) AS source_refs,
	COALESCE(explicit_next_steps, '[]'::jsonb) AS explicit_next_steps,
	COALESCE(inferred_next_steps, '[]'::jsonb) AS inferred_next_steps,
	COALESCE(rejected_options, '[]'::jsonb) AS rejected_options,
	COALESCE(promotion_boundary, '{}'::jsonb) AS promotion_boundary,
	COALESCE(redaction_audit, '{}'::jsonb) AS redaction_audit,
	created_at,
	updated_at
FROM work_journal_entries
WHERE tenant_id = $1 AND entry_id = $2
LIMIT 1",
	)
	.bind(tenant_id)
	.bind(entry_id)
	.fetch_optional(executor)
	.await?;

	Ok(row)
}

/// Lists recent Work Journal entries for one session in newest-first order.
pub async fn list_work_journal_entries_for_session<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	org_project_id: &str,
	session_id: &str,
	max_rows: i64,
) -> Result<Vec<WorkJournalEntry>>
where
	E: PgExecutor<'e>,
{
	let rows = sqlx::query_as::<_, WorkJournalEntry>(
		"\
SELECT
	entry_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	session_id,
	family,
	status,
	title,
	body,
	COALESCE(source_refs, '[]'::jsonb) AS source_refs,
	COALESCE(explicit_next_steps, '[]'::jsonb) AS explicit_next_steps,
	COALESCE(inferred_next_steps, '[]'::jsonb) AS inferred_next_steps,
	COALESCE(rejected_options, '[]'::jsonb) AS rejected_options,
	COALESCE(promotion_boundary, '{}'::jsonb) AS promotion_boundary,
	COALESCE(redaction_audit, '{}'::jsonb) AS redaction_audit,
	created_at,
	updated_at
FROM work_journal_entries
WHERE tenant_id = $1
	AND project_id IN ($2, $3)
	AND session_id = $4
	AND status = 'active'
ORDER BY created_at DESC, entry_id DESC
LIMIT $5",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(org_project_id)
	.bind(session_id)
	.bind(max_rows)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}
