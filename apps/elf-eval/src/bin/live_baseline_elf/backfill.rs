use color_eyre::Result;

use crate::{
	AGENT_ID, AddNoteInput, AddNoteRequest, BACKFILL_CHECKPOINT_SCHEMA, BTreeMap,
	BackfillAttemptEvidence, BackfillCheckpoint, BackfillCheckpointEntry, BackfillOutcome,
	BackfillReport, BackfillResumeReport, CorpusNote, DuplicateSourceNote, ElfService,
	ExistingBackfillNote, Hasher, Instant, NoteOp, PROJECT_ID, Path, PathBuf, SCOPE, TENANT_ID,
	Uuid, env, eyre, fs,
};

pub(super) fn backfill_batch_size() -> usize {
	crate::parse_env_usize("ELF_BASELINE_BACKFILL_BATCH_SIZE").unwrap_or(32).max(1)
}

pub(super) fn worker_concurrency() -> usize {
	let default = match env::var("ELF_BASELINE_PROFILE").as_deref() {
		Ok("backfill" | "large") => 4,
		Ok("stress") => 4,
		Ok("scale" | "full") => 2,
		_ => 1,
	};

	crate::parse_env_usize("ELF_BASELINE_WORKER_CONCURRENCY").unwrap_or(default).clamp(1, 32)
}

pub(super) fn backfill_resume_probe_enabled() -> bool {
	env::var("ELF_BASELINE_BACKFILL_RESUME_PROBE")
		.map(|value| value != "0" && !value.eq_ignore_ascii_case("false"))
		.unwrap_or(true)
}

pub(super) fn backfill_interrupt_after(source_count: usize) -> Option<usize> {
	if !backfill_resume_probe_enabled() || source_count <= 1 {
		return None;
	}

	let configured = crate::parse_env_usize("ELF_BASELINE_BACKFILL_INTERRUPT_AFTER");
	let default = (source_count / 2).max(1);

	Some(configured.unwrap_or(default).clamp(1, source_count.saturating_sub(1)))
}

pub(super) fn backfill_checkpoint_path(out: &Path) -> PathBuf {
	crate::env_string(&["ELF_BASELINE_BACKFILL_CHECKPOINT"])
		.map(PathBuf::from)
		.unwrap_or_else(|| out.with_file_name("elf-backfill-checkpoint.json"))
}

pub(super) fn empty_backfill_checkpoint(corpus_hash: &str) -> BackfillCheckpoint {
	BackfillCheckpoint {
		schema: BACKFILL_CHECKPOINT_SCHEMA.to_string(),
		corpus_hash: corpus_hash.to_string(),
		completed: BTreeMap::new(),
	}
}

pub(super) fn load_backfill_checkpoint(
	path: &Path,
	corpus_hash: &str,
) -> Result<BackfillCheckpoint> {
	if !path.exists() {
		return Ok(empty_backfill_checkpoint(corpus_hash));
	}

	let raw = fs::read_to_string(path)?;
	let checkpoint = serde_json::from_str::<BackfillCheckpoint>(&raw)?;

	if checkpoint.schema == BACKFILL_CHECKPOINT_SCHEMA && checkpoint.corpus_hash == corpus_hash {
		Ok(checkpoint)
	} else {
		Ok(empty_backfill_checkpoint(corpus_hash))
	}
}

pub(super) fn write_backfill_checkpoint(
	path: &Path,
	checkpoint: &BackfillCheckpoint,
) -> Result<()> {
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)?;
	}

	let raw = serde_json::to_string_pretty(checkpoint)?;
	let tmp_path = path.with_extension("json.tmp");

	fs::write(&tmp_path, raw)?;
	fs::rename(tmp_path, path)?;

	Ok(())
}

pub(super) fn source_hash(note: &CorpusNote) -> String {
	let mut hasher = Hasher::new();

	hasher.update(note.source_doc.as_bytes());
	hasher.update(b"\0");
	hasher.update(note.key.as_bytes());
	hasher.update(b"\0");
	hasher.update(note.text.as_bytes());

	hasher.finalize().to_hex().to_string()
}

pub(super) fn corpus_hash(notes: &[CorpusNote]) -> String {
	let mut hasher = Hasher::new();

	for note in notes {
		hasher.update(note.source_doc.as_bytes());
		hasher.update(b"\0");
		hasher.update(source_hash(note).as_bytes());
		hasher.update(b"\0");
	}

	hasher.finalize().to_hex().to_string()
}

pub(super) fn checkpoint_entry_valid(
	note: &CorpusNote,
	entry: &BackfillCheckpointEntry,
	existing: &BTreeMap<String, ExistingBackfillNote>,
) -> bool {
	let expected_hash = source_hash(note);

	if entry.source_hash != expected_hash {
		return false;
	}

	existing.get(&note.source_doc).is_some_and(|stored| {
		stored.note_id == entry.note_id
			&& stored.source_hash.as_deref() == Some(expected_hash.as_str())
	})
}

pub(super) fn note_input(note: &CorpusNote) -> AddNoteInput {
	let hash = source_hash(note);

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

pub(super) async fn run_resumable_backfill(
	service: &ElfService,
	notes: &[CorpusNote],
	checkpoint_path: &Path,
) -> Result<BackfillOutcome> {
	let started_at = Instant::now();
	let corpus_hash = corpus_hash(notes);
	let batch_size = backfill_batch_size();
	let interrupt_after = backfill_interrupt_after(notes.len());
	let first_attempt = run_backfill_attempt(
		service,
		notes,
		checkpoint_path,
		&corpus_hash,
		batch_size,
		1,
		interrupt_after,
	)
	.await?;
	let interrupted = first_attempt.interrupted;
	let completed_before_resume = first_attempt.checkpoint_completed;
	let mut attempts = Vec::new();

	attempts.push(first_attempt);

	if interrupted {
		attempts.push(
			run_backfill_attempt(
				service,
				notes,
				checkpoint_path,
				&corpus_hash,
				batch_size,
				2,
				None,
			)
			.await?,
		);
	}

	let checkpoint = load_backfill_checkpoint(checkpoint_path, &corpus_hash)?;
	let existing = load_existing_backfill_notes(service).await?;
	let mut note_ids = Vec::with_capacity(notes.len());

	for note in notes {
		let Some(entry) = checkpoint.completed.get(&note.source_doc) else {
			return Err(eyre::eyre!(
				"Backfill checkpoint missing completed source {}.",
				note.source_doc
			));
		};

		if !checkpoint_entry_valid(note, entry, &existing) {
			return Err(eyre::eyre!(
				"Backfill checkpoint entry for {} does not match Postgres state.",
				note.source_doc
			));
		}

		note_ids.push(entry.note_id);
	}

	let duplicate_source_notes = duplicate_source_notes(service).await?;
	let attempted_writes = attempts.iter().map(|attempt| attempt.attempted_writes).sum();
	let skipped_completed = attempts.iter().map(|attempt| attempt.skipped_completed).sum();
	let completed_after_resume = checkpoint.completed.len();
	let report = BackfillReport {
		checkpoint_path: checkpoint_path.display().to_string(),
		corpus_hash,
		source_count: notes.len(),
		completed_count: note_ids.len(),
		batch_size,
		worker_concurrency: worker_concurrency(),
		elapsed_seconds: started_at.elapsed().as_secs_f64(),
		attempted_writes,
		skipped_completed,
		duplicate_source_notes,
		resume: BackfillResumeReport {
			enabled: interrupt_after.is_some(),
			interrupted,
			interrupt_after,
			resume_attempts: attempts.len(),
			completed_before_resume,
			completed_after_resume,
		},
		attempts,
	};

	Ok(BackfillOutcome { report, note_ids })
}

pub(super) async fn run_backfill_attempt(
	service: &ElfService,
	notes: &[CorpusNote],
	checkpoint_path: &Path,
	corpus_hash: &str,
	batch_size: usize,
	attempt: usize,
	interrupt_after: Option<usize>,
) -> Result<BackfillAttemptEvidence> {
	let mut checkpoint = load_backfill_checkpoint(checkpoint_path, corpus_hash)?;
	let existing = load_existing_backfill_notes(service).await?;
	let notes_by_source =
		notes.iter().map(|note| (note.source_doc.as_str(), note)).collect::<BTreeMap<_, _>>();
	let checkpoint_len_before_prune = checkpoint.completed.len();

	checkpoint.completed.retain(|source_doc, entry| {
		notes_by_source
			.get(source_doc.as_str())
			.is_some_and(|note| checkpoint_entry_valid(note, entry, &existing))
	});

	if checkpoint.completed.len() != checkpoint_len_before_prune {
		write_backfill_checkpoint(checkpoint_path, &checkpoint)?;
	}

	let mut pending = Vec::new();
	let mut skipped_completed = 0_usize;

	for note in notes {
		if checkpoint.completed.contains_key(&note.source_doc) {
			skipped_completed += 1;
		} else {
			pending.push(note);
		}
	}

	let max_writes = interrupt_after.unwrap_or(usize::MAX);
	let mut attempted_writes = 0_usize;
	let mut completed_writes = 0_usize;
	let mut cursor = 0_usize;

	while cursor < pending.len() && attempted_writes < max_writes {
		let remaining_budget = max_writes.saturating_sub(attempted_writes);
		let take = batch_size.min(remaining_budget).min(pending.len() - cursor);
		let batch = &pending[cursor..cursor + take];
		let response = service
			.add_note(AddNoteRequest {
				tenant_id: TENANT_ID.to_string(),
				project_id: PROJECT_ID.to_string(),
				agent_id: AGENT_ID.to_string(),
				scope: SCOPE.to_string(),
				notes: batch.iter().map(|note| note_input(note)).collect(),
			})
			.await?;

		if response.results.len() != batch.len() {
			return Err(eyre::eyre!(
				"Backfill add_note returned {} results for {} inputs.",
				response.results.len(),
				batch.len()
			));
		}

		for (note, result) in batch.iter().zip(response.results) {
			let op = note_op_string(result.op)?;

			if op == "REJECTED" {
				return Err(eyre::eyre!(
					"Backfill note {} was rejected: {:?}.",
					note.source_doc,
					result.reason_code
				));
			}

			let note_id = result.note_id.ok_or_else(|| {
				eyre::eyre!("Backfill note {} did not return a note_id.", note.source_doc)
			})?;

			checkpoint.completed.insert(
				note.source_doc.clone(),
				BackfillCheckpointEntry {
					note_id,
					key: note.key.clone(),
					source_hash: source_hash(note),
					op,
				},
			);

			completed_writes += 1;
		}

		attempted_writes += batch.len();
		cursor += batch.len();

		write_backfill_checkpoint(checkpoint_path, &checkpoint)?;
	}

	let interrupted = cursor < pending.len();

	Ok(BackfillAttemptEvidence {
		attempt,
		resumed: skipped_completed > 0,
		interrupt_after,
		skipped_completed,
		attempted_writes,
		completed_writes,
		checkpoint_completed: checkpoint.completed.len(),
		interrupted,
	})
}
