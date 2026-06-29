#[path = "validation/adapter.rs"] mod adapter;
#[path = "validation/basics.rs"] mod basics;
#[path = "validation/common.rs"] mod common;
#[path = "validation/consolidation.rs"] mod consolidation;
#[path = "validation/expectations.rs"] mod expectations;
#[path = "validation/job_rules.rs"] mod job_rules;
#[path = "validation/memory_summary.rs"] mod memory_summary;
#[path = "validation/page.rs"] mod page;
#[path = "validation/proactive.rs"] mod proactive;
#[path = "validation/recovery_artifact.rs"] mod recovery_artifact;
#[path = "validation/scheduled.rs"] mod scheduled;
#[path = "validation/trace.rs"] mod trace;
#[path = "validation/work_journal.rs"] mod work_journal;

use self::{
	common::{
		corpus_evidence_ids, corpus_text_by_id, ensure_known_event, ensure_known_evidence,
		ensure_known_evidence_refs, is_memory_summary_category, is_memory_summary_freshness_status,
		is_memory_summary_rationale_decision, is_proactive_action_decision,
		is_proactive_suggestion_kind, is_scheduled_task_kind, timeline_event_ids,
		validate_optional_rfc3339, validate_optional_summary_time, validate_required_rfc3339,
	},
	memory_summary::{validate_memory_summary_artifact, validate_memory_summary_source_trace},
	page::validate_page_artifact,
	proactive::validate_proactive_brief_artifact,
	recovery_artifact::validate_authority_recovery_drill_artifact,
	scheduled::validate_scheduled_memory_artifact,
	work_journal::validate_work_journal_readback_artifact,
};
use crate::{
	AUTHORITY_RECOVERY_DRILL_SCHEMA, AuthorityRecoveryDrillArtifact, BTreeMap, BTreeSet,
	ConsolidationProposalFixture, DerivedPageArtifact, EvolutionConflict, JOB_SCHEMA,
	MemorySummaryArtifact, MemorySummaryEntry, MemorySummarySourceTrace, OffsetDateTime, Path,
	ProactiveBriefArtifact, ProactiveSuggestion, RealWorldJob, RecoveryBackupPitr,
	RecoveryDeadLetterHandling, RecoveryDegradedRead, RecoveryDrillTopology, RecoveryMeasurement,
	RecoveryMigrationRepair, RecoveryOutboxReplay, RecoveryQdrantRebuild, Result, Rfc3339, SUITES,
	ScheduledMemoryExecutionTrace, ScheduledMemoryOutput, ScheduledMemoryTaskArtifact,
	TemporalValidity, TraceStageExplainability, TypedStatus, UpdateRationale, Value,
	WorkJournalEntryArtifact, WorkJournalNextStepArtifact, WorkJournalReadbackArtifact,
	WorkJournalWhereStoppedArtifact, eyre,
	formatting::status_str,
	recovery::{
		REQUIRED_AUTHORITY_PLANES, recovery_dead_letter_succeeded, recovery_measurement_met,
		recovery_migration_repair_succeeded, recovery_outbox_replay_succeeded,
		recovery_qdrant_rebuild_succeeded,
	},
};

pub(super) fn validate_job(job: &RealWorldJob, path: &Path) -> Result<()> {
	if job.schema != JOB_SCHEMA {
		return Err(eyre::eyre!(
			"{} has schema {}, expected {JOB_SCHEMA}.",
			path.display(),
			job.schema
		));
	}

	self::basics::validate_job_identity(job, path)?;

	if !SUITES.contains(&job.suite.as_str()) {
		return Err(eyre::eyre!("{} uses unknown suite {}.", path.display(), job.suite));
	}

	self::basics::validate_corpus_items(job, path)?;
	self::basics::validate_timeline(job, path)?;
	self::basics::validate_prompt(job, path)?;
	self::basics::validate_expected_answer(job, path)?;
	self::basics::validate_required_evidence(job, path)?;
	self::consolidation::validate_consolidation_fixture(job, path)?;
	self::adapter::validate_adapter_response(job, path)?;
	self::job_rules::validate_scoring_rubric(job, path)?;
	self::job_rules::validate_allowed_uncertainty(job, path)?;
	self::job_rules::validate_operator_debug(job, path)?;
	self::job_rules::validate_job_encoding(job, path)?;
	self::expectations::validate_memory_evolution(job, path)?;
	self::expectations::validate_memory_summary_expectation(job, path)?;
	self::expectations::validate_proactive_brief_expectation(job, path)?;
	self::expectations::validate_scheduled_memory_expectation(job, path)?;
	self::expectations::validate_work_continuity_expectation(job, path)?;
	self::trace::validate_trace_explainability(job, path)?;

	Ok(())
}
