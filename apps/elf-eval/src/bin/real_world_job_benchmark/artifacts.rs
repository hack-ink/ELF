#[path = "artifacts/answer.rs"] mod answer;
#[path = "artifacts/consolidation.rs"] mod consolidation;
#[path = "artifacts/cost.rs"] mod cost;
#[path = "artifacts/knowledge.rs"] mod knowledge;
#[path = "artifacts/memory.rs"] mod memory;
#[path = "artifacts/proactive.rs"] mod proactive;
#[path = "artifacts/recovery.rs"] mod recovery;
#[path = "artifacts/scheduled.rs"] mod scheduled;
#[path = "artifacts/work.rs"] mod work;

pub(super) use self::{
	answer::{ProducedAnswer, ProducedClaim},
	consolidation::{ConsolidationFixture, ConsolidationProposalFixture},
	cost::CostReport,
	knowledge::{DerivedPageArtifact, DerivedPageRebuild, DerivedPageSection},
	memory::{MemorySummaryArtifact, MemorySummaryEntry, MemorySummarySourceTrace},
	proactive::{ProactiveBriefArtifact, ProactiveSuggestion},
	recovery::{
		AuthorityRecordCount, AuthorityRecoveryDrillArtifact, RecoveryBackupPitr,
		RecoveryDeadLetterHandling, RecoveryDegradedRead, RecoveryDrillTopology,
		RecoveryMeasurement, RecoveryMigrationRepair, RecoveryOutboxReplay, RecoveryQdrantRebuild,
	},
	scheduled::{
		ScheduledMemoryExecutionTrace, ScheduledMemoryOutput, ScheduledMemoryTaskArtifact,
	},
	work::{
		WorkContinuityObserved, WorkJournalEntryArtifact, WorkJournalJanitorCandidateArtifact,
		WorkJournalNextStepArtifact, WorkJournalReadbackArtifact,
		WorkJournalRejectedOptionArtifact, WorkJournalWhereStoppedArtifact,
	},
};
