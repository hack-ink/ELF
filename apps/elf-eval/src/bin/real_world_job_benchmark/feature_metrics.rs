#[path = "feature_metrics/common.rs"] mod common;
#[path = "feature_metrics/knowledge.rs"] mod knowledge;
#[path = "feature_metrics/memory_summary.rs"] mod memory_summary;
#[path = "feature_metrics/proactive.rs"] mod proactive;
#[path = "feature_metrics/scheduled.rs"] mod scheduled;
#[path = "feature_metrics/work_continuity.rs"] mod work_continuity;

use crate::{
	BTreeSet, DerivedPageArtifact, DerivedPageRebuild, DerivedPageSection,
	FORBIDDEN_SOURCE_MUTATION_KEYS, KnowledgeJobMetrics, MemorySummaryArtifact, MemorySummaryEntry,
	MemorySummaryJobMetrics, MemorySummarySourceTrace, NegativeTrap, ProactiveBriefArtifact,
	ProactiveBriefJobMetrics, ProactiveSuggestion, ProducedAnswer, RealWorldJob,
	ScheduledMemoryExecutionTrace, ScheduledMemoryJobMetrics, ScheduledMemoryOutput,
	ScheduledMemoryTaskArtifact, UnsupportedClaimReport, Value, WorkContinuityExpectation,
	WorkContinuityJobMetrics, WorkContinuityObserved, WorkJournalJanitorCandidateArtifact,
	WorkJournalNextStepArtifact, WorkJournalReadbackArtifact, WorkJournalRejectedOptionArtifact,
	formatting::{bounded_text, round3},
	summary::{ratio, ratio_or, ratio_or_full},
};

pub(super) fn unsupported_page_claims(answer: &ProducedAnswer) -> Vec<UnsupportedClaimReport> {
	knowledge::unsupported_page_claims_impl(answer)
}

pub(super) fn knowledge_metrics(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
) -> Option<KnowledgeJobMetrics> {
	knowledge::knowledge_metrics_impl(job, answer)
}

pub(super) fn missed_stale_finding_count(metrics: &KnowledgeJobMetrics) -> usize {
	knowledge::missed_stale_finding_count_impl(metrics)
}

pub(super) fn page_usefulness_failure_count(metrics: &KnowledgeJobMetrics) -> usize {
	knowledge::page_usefulness_failure_count_impl(metrics)
}

pub(super) fn memory_summary_metrics(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
) -> Option<MemorySummaryJobMetrics> {
	memory_summary::memory_summary_metrics_impl(job, answer)
}

pub(super) fn unsupported_memory_summary_claims(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
) -> Vec<UnsupportedClaimReport> {
	memory_summary::unsupported_memory_summary_claims_impl(job, answer)
}

pub(super) fn proactive_brief_metrics(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
) -> Option<ProactiveBriefJobMetrics> {
	proactive::proactive_brief_metrics_impl(job, answer)
}

pub(super) fn unsupported_proactive_suggestions(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
) -> Vec<UnsupportedClaimReport> {
	proactive::unsupported_proactive_suggestions_impl(job, answer)
}

pub(super) fn scheduled_memory_metrics(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
) -> Option<ScheduledMemoryJobMetrics> {
	scheduled::scheduled_memory_metrics_impl(job, answer)
}

pub(super) fn unsupported_scheduled_outputs(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
) -> Vec<UnsupportedClaimReport> {
	scheduled::unsupported_scheduled_outputs_impl(job, answer)
}

pub(super) fn work_continuity_metrics(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
) -> Option<WorkContinuityJobMetrics> {
	work_continuity::work_continuity_metrics_impl(job, answer)
}

pub(super) fn forbidden_diff_key_count(value: &Value) -> usize {
	common::forbidden_diff_key_count_impl(value)
}

fn memory_summary_non_current_trace_refs(trace: &MemorySummarySourceTrace) -> BTreeSet<&str> {
	memory_summary::memory_summary_non_current_trace_refs_impl(trace)
}

fn proactive_tombstone_trace_refs(trace: &MemorySummarySourceTrace) -> BTreeSet<&str> {
	proactive::proactive_tombstone_trace_refs_impl(trace)
}
