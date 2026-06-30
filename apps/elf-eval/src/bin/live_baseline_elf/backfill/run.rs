use color_eyre::Result;

use crate::{
	AGENT_ID, AddNoteRequest, BTreeMap, BackfillAttemptEvidence, BackfillCheckpointEntry,
	BackfillOutcome, BackfillReport, BackfillResumeReport, CorpusNote, ElfService, Instant,
	PROJECT_ID, Path, SCOPE, TENANT_ID,
	backfill::{backfill_checkpoint, config, notes},
	eyre,
};

pub(crate) async fn run_resumable_backfill(
	service: &ElfService,
	notes: &[CorpusNote],
	checkpoint_path: &Path,
) -> Result<BackfillOutcome> {
	let started_at = Instant::now();
	let corpus_hash = backfill_checkpoint::corpus_hash(notes);
	let batch_size = config::backfill_batch_size();
	let interrupt_after = config::backfill_interrupt_after(notes.len());
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

	let checkpoint = backfill_checkpoint::load_backfill_checkpoint(checkpoint_path, &corpus_hash)?;
	let existing = notes::load_existing_backfill_notes(service).await?;
	let mut note_ids = Vec::with_capacity(notes.len());

	for note in notes {
		let Some(entry) = checkpoint.completed.get(&note.source_doc) else {
			return Err(eyre::eyre!(
				"Backfill checkpoint missing completed source {}.",
				note.source_doc
			));
		};

		if !backfill_checkpoint::checkpoint_entry_valid(note, entry, &existing) {
			return Err(eyre::eyre!(
				"Backfill checkpoint entry for {} does not match Postgres state.",
				note.source_doc
			));
		}

		note_ids.push(entry.note_id);
	}

	let duplicate_source_notes = notes::duplicate_source_notes(service).await?;
	let attempted_writes = attempts.iter().map(|attempt| attempt.attempted_writes).sum();
	let skipped_completed = attempts.iter().map(|attempt| attempt.skipped_completed).sum();
	let completed_after_resume = checkpoint.completed.len();
	let report = BackfillReport {
		checkpoint_path: checkpoint_path.display().to_string(),
		corpus_hash,
		source_count: notes.len(),
		completed_count: note_ids.len(),
		batch_size,
		worker_concurrency: config::worker_concurrency(),
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

async fn run_backfill_attempt(
	service: &ElfService,
	notes: &[CorpusNote],
	checkpoint_path: &Path,
	corpus_hash: &str,
	batch_size: usize,
	attempt: usize,
	interrupt_after: Option<usize>,
) -> Result<BackfillAttemptEvidence> {
	let mut checkpoint =
		backfill_checkpoint::load_backfill_checkpoint(checkpoint_path, corpus_hash)?;
	let existing = notes::load_existing_backfill_notes(service).await?;
	let notes_by_source =
		notes.iter().map(|note| (note.source_doc.as_str(), note)).collect::<BTreeMap<_, _>>();
	let checkpoint_len_before_prune = checkpoint.completed.len();

	checkpoint.completed.retain(|source_doc, entry| {
		notes_by_source
			.get(source_doc.as_str())
			.is_some_and(|note| backfill_checkpoint::checkpoint_entry_valid(note, entry, &existing))
	});

	if checkpoint.completed.len() != checkpoint_len_before_prune {
		backfill_checkpoint::write_backfill_checkpoint(checkpoint_path, &checkpoint)?;
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
				notes: batch.iter().map(|note| notes::note_input(note)).collect(),
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
			let op = notes::note_op_string(result.op)?;

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
					source_hash: backfill_checkpoint::source_hash(note),
					op,
				},
			);

			completed_writes += 1;
		}

		attempted_writes += batch.len();
		cursor += batch.len();

		backfill_checkpoint::write_backfill_checkpoint(checkpoint_path, &checkpoint)?;
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
