use super::*;

/// Response returned after rebuilding pages affected by changed sources.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageWatchRebuildResponse {
	/// Versioned response schema.
	pub schema: String,
	/// Operator-readable aggregate summary.
	pub summary: KnowledgePageWatchRebuildSummary,
	/// Per-page rebuild results.
	pub pages: Vec<KnowledgePageWatchRebuildItem>,
	/// Reviewable memory candidates derived from knowledge deltas.
	pub memory_candidates: Vec<KnowledgeDeltaMemoryCandidate>,
	/// Queued consolidation run, when memory candidates were generated.
	pub proposal_run: Option<KnowledgePageProposalRunSummary>,
	/// One-line operator summary messages.
	pub operator_summary: Vec<String>,
}

/// Aggregate watch/rebuild outcome counters.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageWatchRebuildSummary {
	/// Changed source count after de-duplication.
	pub changed_source_count: usize,
	/// Knowledge pages that cited one of the changed sources.
	pub affected_page_count: usize,
	/// Pages rebuilt with changed derived output.
	pub changed_page_count: usize,
	/// Pages rebuilt with unchanged derived output.
	pub unchanged_page_count: usize,
	/// Pages that had stale lint findings before rebuild.
	pub stale_page_count: usize,
	/// Pages that could not be rebuilt.
	pub blocked_page_count: usize,
	/// Memory candidates generated for review.
	pub memory_candidate_count: usize,
}

/// Per-page changed-source rebuild result.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageWatchRebuildItem {
	/// Knowledge page identifier.
	pub page_id: Uuid,
	/// Page kind.
	pub page_kind: String,
	/// Stable page key.
	pub page_key: String,
	/// Page title.
	pub title: String,
	/// Page rebuild state: changed, unchanged, stale, or blocked.
	pub rebuild_state: String,
	/// Per-section rebuild states.
	pub sections: Vec<KnowledgePageSectionRebuildState>,
	/// Classified rebuild/lint outputs.
	pub outputs: Vec<KnowledgePageRebuildOutput>,
	/// Rebuilt page readback, omitted when blocked.
	pub rebuilt_page: Option<KnowledgePageResponse>,
	/// Blocking error text, when rebuild failed.
	pub blocked_reason: Option<String>,
	/// Previous-version diff metadata, when available.
	pub previous_version_diff: Option<Value>,
	/// Operator-readable page summary.
	pub operator_summary: String,
}

/// Per-section rebuild state for changed-source rebuild output.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageSectionRebuildState {
	/// Stable section key.
	pub section_key: String,
	/// Section heading.
	pub heading: String,
	/// Section state: changed, unchanged, stale, or blocked.
	pub state: String,
	/// Output types attached to the section.
	pub output_types: Vec<String>,
	/// Lint finding types attached to the section before rebuild.
	pub lint_finding_types: Vec<String>,
}

/// Classified output emitted by the watch/rebuild loop.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageRebuildOutput {
	/// Output type, such as stale_section, changed_claim, missing_citation, conflict,
	/// changed_source, or blocked.
	pub output_type: String,
	/// Severity for operator triage.
	pub severity: String,
	/// Associated section key, when section-scoped.
	pub section_key: Option<String>,
	/// Associated source kind, when source-scoped.
	pub source_kind: Option<String>,
	/// Associated source id, when source-scoped.
	pub source_id: Option<Uuid>,
	/// Human-readable output message.
	pub message: String,
	/// Structured reason and evidence details.
	pub details: Value,
}

/// Reviewable memory candidate produced from a knowledge delta.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgeDeltaMemoryCandidate {
	/// Candidate reason, such as changed_claim or conflict.
	pub reason: String,
	/// Knowledge page identifier.
	pub page_id: Uuid,
	/// Section identifier that produced the candidate.
	pub section_id: Uuid,
	/// Stable section key.
	pub section_key: String,
	/// Source refs copied into the reviewable proposal.
	pub source_refs: Vec<ConsolidationInputRef>,
	/// Source snapshot summary for reviewer inspection.
	pub source_snapshot: Value,
	/// Reviewable proposal diff.
	pub diff: ConsolidationProposalDiff,
	/// Proposed memory note payload.
	pub proposed_payload: Value,
}

/// Queued reviewable proposal run produced by changed-source rebuild.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageProposalRunSummary {
	/// Consolidation run identifier.
	pub run_id: Uuid,
	/// Queued worker job identifier.
	pub job_id: Uuid,
	/// Number of memory candidate proposals queued in the run payload.
	pub proposal_count: usize,
	/// Review surface for the queued candidates.
	pub review_surface: String,
}
