use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{Error, Result, access::ORG_PROJECT_ID};
use elf_storage::models::MemoryNote;

pub(in crate::memory_corrections) async fn load_note_for_correction(
	tx: &mut Transaction<'_, Postgres>,
	note_id: Uuid,
	tenant_id: &str,
	project_id: &str,
) -> Result<MemoryNote> {
	sqlx::query_as::<_, MemoryNote>(
		"\
SELECT *
FROM memory_notes
WHERE note_id = $1 AND tenant_id = $2 AND project_id IN ($3, $4)
FOR UPDATE",
	)
	.bind(note_id)
	.bind(tenant_id)
	.bind(project_id)
	.bind(ORG_PROJECT_ID)
	.fetch_optional(&mut **tx)
	.await?
	.ok_or_else(|| Error::InvalidRequest { message: "Note not found.".to_string() })
}
