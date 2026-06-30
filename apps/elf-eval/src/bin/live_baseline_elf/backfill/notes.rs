use color_eyre::Result;

use crate::{
	AGENT_ID, AddNoteInput, BTreeMap, CorpusNote, DuplicateSourceNote, ElfService,
	ExistingBackfillNote, NoteOp, PROJECT_ID, SCOPE, TENANT_ID, Uuid,
	backfill::backfill_checkpoint, eyre,
};

pub(super) fn note_input(note: &CorpusNote) -> AddNoteInput {
	let hash = backfill_checkpoint::source_hash(note);

	AddNoteInput {
		r#type: "fact".to_string(),
		key: Some(note.key.clone()),
		text: note.text.clone(),
		structured: None,
		importance: 0.9,
		confidence: 0.95,
		ttl_days: None,
		source_ref: serde_json::json!({
			"source": "ELF live baseline corpus",
			"title": note.title,
			"document": note.source_doc,
			"source_hash": hash,
		}),
		write_policy: None,
	}
}

pub(super) fn note_op_string(op: NoteOp) -> Result<String> {
	let value = serde_json::to_value(op)?;

	value
		.as_str()
		.map(ToString::to_string)
		.ok_or_else(|| eyre::eyre!("Serialized note op was not a string."))
}

pub(super) async fn load_existing_backfill_notes(
	service: &ElfService,
) -> Result<BTreeMap<String, ExistingBackfillNote>> {
	let rows = sqlx::query_as::<_, (Uuid, String, Option<String>)>(
		"\
SELECT note_id, source_ref->>'document' AS source_doc, source_ref->>'source_hash' AS source_hash
FROM memory_notes
WHERE tenant_id = $1
	AND project_id = $2
	AND agent_id = $3
	AND scope = $4
	AND status = 'active'
	AND source_ref->>'source' = 'ELF live baseline corpus'
	AND source_ref->>'document' IS NOT NULL
ORDER BY updated_at DESC",
	)
	.bind(TENANT_ID)
	.bind(PROJECT_ID)
	.bind(AGENT_ID)
	.bind(SCOPE)
	.fetch_all(&service.db.pool)
	.await?;
	let mut out = BTreeMap::new();

	for (note_id, source_doc, hash) in rows {
		out.entry(source_doc).or_insert(ExistingBackfillNote { note_id, source_hash: hash });
	}

	Ok(out)
}

pub(super) async fn duplicate_source_notes(
	service: &ElfService,
) -> Result<Vec<DuplicateSourceNote>> {
	let rows = sqlx::query_as::<_, (String, i64, Vec<Uuid>)>(
		"\
SELECT
	source_ref->>'document' AS source_doc,
	COUNT(*)::bigint AS count,
	array_agg(note_id ORDER BY note_id)::uuid[] AS note_ids
FROM memory_notes
WHERE tenant_id = $1
	AND project_id = $2
	AND agent_id = $3
	AND scope = $4
	AND status = 'active'
	AND source_ref->>'source' = 'ELF live baseline corpus'
	AND source_ref->>'document' IS NOT NULL
GROUP BY source_ref->>'document'
HAVING COUNT(*) > 1
ORDER BY source_doc",
	)
	.bind(TENANT_ID)
	.bind(PROJECT_ID)
	.bind(AGENT_ID)
	.bind(SCOPE)
	.fetch_all(&service.db.pool)
	.await?;

	Ok(rows
		.into_iter()
		.map(|(source_doc, count, note_ids)| DuplicateSourceNote { source_doc, count, note_ids })
		.collect())
}
