use crate::{BTreeMap, Deserialize, Serialize, Uuid};

#[derive(Debug)]
pub(crate) struct BackfillOutcome {
	pub(crate) report: BackfillReport,
	pub(crate) note_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub(crate) struct ExistingBackfillNote {
	pub(crate) note_id: Uuid,
	pub(crate) source_hash: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct BackfillCheckpoint {
	pub(crate) schema: String,
	pub(crate) corpus_hash: String,
	pub(crate) completed: BTreeMap<String, BackfillCheckpointEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct BackfillCheckpointEntry {
	pub(crate) note_id: Uuid,
	pub(crate) key: String,
	pub(crate) source_hash: String,
	pub(crate) op: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct BackfillReport {
	pub(crate) checkpoint_path: String,
	pub(crate) corpus_hash: String,
	pub(crate) source_count: usize,
	pub(crate) completed_count: usize,
	pub(crate) batch_size: usize,
	pub(crate) worker_concurrency: usize,
	pub(crate) elapsed_seconds: f64,
	pub(crate) attempted_writes: usize,
	pub(crate) skipped_completed: usize,
	pub(crate) duplicate_source_notes: Vec<DuplicateSourceNote>,
	pub(crate) resume: BackfillResumeReport,
	pub(crate) attempts: Vec<BackfillAttemptEvidence>,
}

#[derive(Debug, Serialize)]
pub(crate) struct BackfillResumeReport {
	pub(crate) enabled: bool,
	pub(crate) interrupted: bool,
	pub(crate) interrupt_after: Option<usize>,
	pub(crate) resume_attempts: usize,
	pub(crate) completed_before_resume: usize,
	pub(crate) completed_after_resume: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct BackfillAttemptEvidence {
	pub(crate) attempt: usize,
	pub(crate) resumed: bool,
	pub(crate) interrupt_after: Option<usize>,
	pub(crate) skipped_completed: usize,
	pub(crate) attempted_writes: usize,
	pub(crate) completed_writes: usize,
	pub(crate) checkpoint_completed: usize,
	pub(crate) interrupted: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct DuplicateSourceNote {
	pub(crate) source_doc: String,
	pub(crate) count: i64,
	pub(crate) note_ids: Vec<Uuid>,
}
