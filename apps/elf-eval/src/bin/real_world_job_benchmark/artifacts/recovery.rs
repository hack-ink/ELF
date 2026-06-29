use crate::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AuthorityRecoveryDrillArtifact {
	pub(crate) drill_id: String,
	pub(crate) contract_schema: String,
	pub(crate) generated_at: String,
	pub(crate) topology: RecoveryDrillTopology,
	#[serde(default)]
	pub(crate) failure_injections: Vec<RecoveryFailureInjection>,
	pub(crate) backup_pitr: RecoveryBackupPitr,
	pub(crate) degraded_read: RecoveryDegradedRead,
	pub(crate) rpo: RecoveryMeasurement,
	pub(crate) rto: RecoveryMeasurement,
	#[serde(default)]
	pub(crate) authority_record_counts: Vec<AuthorityRecordCount>,
	pub(crate) outbox_replay: RecoveryOutboxReplay,
	pub(crate) qdrant_rebuild: RecoveryQdrantRebuild,
	pub(crate) migration_repair: RecoveryMigrationRepair,
	pub(crate) dead_letter: RecoveryDeadLetterHandling,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RecoveryDrillTopology {
	pub(crate) authority_store: String,
	#[serde(default)]
	pub(crate) derived_indexes: Vec<String>,
	#[serde(default)]
	pub(crate) adapters: Vec<String>,
	pub(crate) failover: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RecoveryFailureInjection {
	pub(crate) injection_id: String,
	pub(crate) target: String,
	pub(crate) fault: String,
	pub(crate) started_at: String,
	pub(crate) completed_at: String,
	#[serde(default)]
	pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RecoveryBackupPitr {
	pub(crate) backup_ref: String,
	pub(crate) pitr_target: String,
	pub(crate) restored: bool,
	#[serde(default)]
	pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RecoveryDegradedRead {
	pub(crate) source_of_truth_visible: bool,
	#[serde(default)]
	pub(crate) unavailable_derived_indexes: Vec<String>,
	#[serde(default)]
	pub(crate) unavailable_adapters: Vec<String>,
	#[serde(default)]
	pub(crate) unavailable_labels: Vec<String>,
	#[serde(default)]
	pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RecoveryMeasurement {
	pub(crate) target_seconds: f64,
	pub(crate) measured_seconds: f64,
	#[serde(default)]
	pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AuthorityRecordCount {
	pub(crate) plane: String,
	pub(crate) before_count: u64,
	pub(crate) after_count: u64,
	pub(crate) source_refs_preserved: bool,
	pub(crate) lifecycle_history_preserved: bool,
	#[serde(default)]
	pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RecoveryOutboxReplay {
	pub(crate) idempotent: bool,
	pub(crate) replayed_count: u64,
	pub(crate) duplicate_write_count: u64,
	#[serde(default)]
	pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RecoveryQdrantRebuild {
	pub(crate) complete: bool,
	pub(crate) rebuilt_count: u64,
	pub(crate) missing_vector_count: u64,
	pub(crate) error_count: u64,
	#[serde(default)]
	pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RecoveryMigrationRepair {
	pub(crate) applied: bool,
	pub(crate) repaired_count: u64,
	#[serde(default)]
	pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RecoveryDeadLetterHandling {
	pub(crate) dead_letter_count: u64,
	pub(crate) handled_count: u64,
	#[serde(default)]
	pub(crate) evidence_refs: Vec<String>,
}
