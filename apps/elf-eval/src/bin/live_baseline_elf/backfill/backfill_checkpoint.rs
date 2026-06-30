use color_eyre::Result;

use crate::{
	BACKFILL_CHECKPOINT_SCHEMA, BTreeMap, BackfillCheckpoint, BackfillCheckpointEntry, CorpusNote,
	ExistingBackfillNote, Hasher, Path, PathBuf, fs,
};

pub(super) fn backfill_checkpoint_path(out: &Path) -> PathBuf {
	crate::env_string(&["ELF_BASELINE_BACKFILL_CHECKPOINT"])
		.map(PathBuf::from)
		.unwrap_or_else(|| out.with_file_name("elf-backfill-checkpoint.json"))
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

fn empty_backfill_checkpoint(corpus_hash: &str) -> BackfillCheckpoint {
	BackfillCheckpoint {
		schema: BACKFILL_CHECKPOINT_SCHEMA.to_string(),
		corpus_hash: corpus_hash.to_string(),
		completed: BTreeMap::new(),
	}
}

#[cfg(test)]
mod tests {
	use std::env;

	use uuid::Uuid;

	use crate::{
		BACKFILL_CHECKPOINT_SCHEMA, BTreeMap, BackfillCheckpoint, BackfillCheckpointEntry,
		CorpusNote, ExistingBackfillNote, backfill::backfill_checkpoint, fs,
	};

	fn note(source_doc: &str, key: &str, text: &str) -> CorpusNote {
		CorpusNote {
			key: key.to_string(),
			title: format!("Title {source_doc}"),
			text: text.to_string(),
			source_doc: source_doc.to_string(),
		}
	}

	#[test]
	fn source_and_corpus_hashes_are_deterministic_and_content_sensitive() {
		let baseline = note("doc-a.md", "alpha", "Current project decision.");
		let same = note("doc-a.md", "alpha", "Current project decision.");
		let changed_doc = note("doc-b.md", "alpha", "Current project decision.");
		let changed_key = note("doc-a.md", "beta", "Current project decision.");
		let changed_text = note("doc-a.md", "alpha", "Updated project decision.");

		assert_eq!(
			backfill_checkpoint::source_hash(&baseline),
			backfill_checkpoint::source_hash(&same)
		);
		assert_ne!(
			backfill_checkpoint::source_hash(&baseline),
			backfill_checkpoint::source_hash(&changed_doc)
		);
		assert_ne!(
			backfill_checkpoint::source_hash(&baseline),
			backfill_checkpoint::source_hash(&changed_key)
		);
		assert_ne!(
			backfill_checkpoint::source_hash(&baseline),
			backfill_checkpoint::source_hash(&changed_text)
		);
		assert_eq!(
			backfill_checkpoint::corpus_hash(&[baseline, changed_doc]),
			backfill_checkpoint::corpus_hash(&[
				same,
				note("doc-b.md", "alpha", "Current project decision.")
			])
		);
		assert_ne!(
			backfill_checkpoint::corpus_hash(&[
				note("doc-a.md", "alpha", "Current project decision."),
				changed_key
			]),
			backfill_checkpoint::corpus_hash(&[
				note("doc-a.md", "alpha", "Current project decision."),
				changed_text
			])
		);
	}

	#[test]
	fn checkpoint_load_resets_missing_schema_or_corpus_mismatches() {
		let root = env::temp_dir().join(format!("elf-backfill-checkpoint-test-{}", Uuid::new_v4()));
		let path = root.join("checkpoint.json");
		let missing = backfill_checkpoint::load_backfill_checkpoint(&path, "corpus-a")
			.expect("Missing checkpoint loads.");

		assert_eq!(missing.schema, BACKFILL_CHECKPOINT_SCHEMA);
		assert_eq!(missing.corpus_hash, "corpus-a");
		assert!(missing.completed.is_empty());

		fs::create_dir_all(&root).expect("Temp dir is created.");
		fs::write(
			&path,
			r#"{"schema":"old","corpus_hash":"corpus-a","completed":{"doc.md":{"note_id":"00000000-0000-0000-0000-000000000001","key":"k","source_hash":"h","op":"CREATED"}}}"#,
		)
		.expect("Mismatched checkpoint is written.");

		let schema_reset = backfill_checkpoint::load_backfill_checkpoint(&path, "corpus-a")
			.expect("Checkpoint loads.");

		assert!(schema_reset.completed.is_empty());

		fs::write(
			&path,
			r#"{"schema":"elf.live_baseline.backfill_checkpoint/v1","corpus_hash":"corpus-b","completed":{"doc.md":{"note_id":"00000000-0000-0000-0000-000000000001","key":"k","source_hash":"h","op":"CREATED"}}}"#,
		)
		.expect("Mismatched checkpoint is written.");

		let corpus_reset = backfill_checkpoint::load_backfill_checkpoint(&path, "corpus-a")
			.expect("Checkpoint loads.");

		assert!(corpus_reset.completed.is_empty());

		fs::remove_dir_all(root).expect("Temp dir is removed.");
	}

	#[test]
	fn checkpoint_write_round_trips_through_parent_directories() {
		let root = env::temp_dir().join(format!("elf-backfill-checkpoint-test-{}", Uuid::new_v4()));
		let path = root.join("nested").join("checkpoint.json");
		let note_id = Uuid::new_v4();
		let mut checkpoint = BackfillCheckpoint {
			schema: BACKFILL_CHECKPOINT_SCHEMA.to_string(),
			corpus_hash: "corpus-a".to_string(),
			completed: BTreeMap::new(),
		};

		checkpoint.completed.insert(
			"doc.md".to_string(),
			BackfillCheckpointEntry {
				note_id,
				key: "k".to_string(),
				source_hash: "h".to_string(),
				op: "CREATED".to_string(),
			},
		);

		backfill_checkpoint::write_backfill_checkpoint(&path, &checkpoint)
			.expect("Checkpoint writes.");

		let loaded = backfill_checkpoint::load_backfill_checkpoint(&path, "corpus-a")
			.expect("Checkpoint reloads.");

		assert_eq!(loaded.completed["doc.md"].note_id, note_id);
		assert!(fs::read_to_string(&path).expect("Checkpoint is readable.").contains('\n'));

		fs::remove_dir_all(root).expect("Temp dir is removed.");
	}

	#[test]
	fn checkpoint_entry_validation_requires_matching_hash_note_id_and_existing_source() {
		let note = note("doc.md", "k", "text");
		let note_id = Uuid::new_v4();
		let source_hash = backfill_checkpoint::source_hash(&note);
		let entry = BackfillCheckpointEntry {
			note_id,
			key: "k".to_string(),
			source_hash: source_hash.clone(),
			op: "CREATED".to_string(),
		};
		let mut existing = BTreeMap::new();

		existing.insert(
			"doc.md".to_string(),
			ExistingBackfillNote { note_id, source_hash: Some(source_hash) },
		);

		assert!(backfill_checkpoint::checkpoint_entry_valid(&note, &entry, &existing));

		let mut stale_entry = entry.clone();

		stale_entry.source_hash = "stale".to_string();

		assert!(!backfill_checkpoint::checkpoint_entry_valid(&note, &stale_entry, &existing));

		let mut wrong_note_id = existing;

		wrong_note_id.insert(
			"doc.md".to_string(),
			ExistingBackfillNote {
				note_id: Uuid::new_v4(),
				source_hash: Some(entry.source_hash.clone()),
			},
		);

		assert!(!backfill_checkpoint::checkpoint_entry_valid(&note, &entry, &wrong_note_id));
		assert!(!backfill_checkpoint::checkpoint_entry_valid(&note, &entry, &BTreeMap::new()));

		let mut missing_stored_hash = BTreeMap::new();

		missing_stored_hash
			.insert("doc.md".to_string(), ExistingBackfillNote { note_id, source_hash: None });

		assert!(!backfill_checkpoint::checkpoint_entry_valid(&note, &entry, &missing_stored_hash));

		let mut mismatched_stored_hash = BTreeMap::new();

		mismatched_stored_hash.insert(
			"doc.md".to_string(),
			ExistingBackfillNote { note_id, source_hash: Some("stale".to_string()) },
		);

		assert!(!backfill_checkpoint::checkpoint_entry_valid(
			&note,
			&entry,
			&mismatched_stored_hash
		));
	}
}
