use crate::{BTreeSet, Deserialize, Serialize};

pub(crate) struct WorkContinuityObserved<'a> {
	pub(crate) reset_resume_entry_ids: BTreeSet<&'a str>,
	pub(crate) decision_rationale_evidence_ids: BTreeSet<&'a str>,
	pub(crate) rejected_options: Vec<&'a WorkJournalRejectedOptionArtifact>,
	pub(crate) explicit_next_steps: Vec<&'a WorkJournalNextStepArtifact>,
	pub(crate) inferred_next_steps: Vec<&'a WorkJournalNextStepArtifact>,
	pub(crate) handoff_source_refs: BTreeSet<&'a str>,
	pub(crate) redacted_marker_ids: BTreeSet<&'a str>,
	pub(crate) janitor_candidates: Vec<&'a WorkJournalJanitorCandidateArtifact>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct WorkJournalReadbackArtifact {
	pub(crate) readback_id: String,
	pub(crate) contract_schema: String,
	pub(crate) generated_at: String,
	pub(crate) session_id: String,
	pub(crate) tenant_id: String,
	pub(crate) project_id: String,
	pub(crate) agent_id: String,
	pub(crate) read_profile: String,
	#[serde(default)]
	pub(crate) items: Vec<WorkJournalEntryArtifact>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) where_stopped: Option<WorkJournalWhereStoppedArtifact>,
	pub(crate) promotion_boundary: WorkJournalPromotionBoundaryArtifact,
	#[serde(default)]
	pub(crate) janitor_candidates: Vec<WorkJournalJanitorCandidateArtifact>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct WorkJournalEntryArtifact {
	pub(crate) entry_id: String,
	pub(crate) family: String,
	pub(crate) title: String,
	pub(crate) body: String,
	#[serde(default)]
	pub(crate) source_refs: Vec<String>,
	#[serde(default)]
	pub(crate) redaction_audit: WorkJournalRedactionAuditArtifact,
	#[serde(default)]
	pub(crate) explicit_next_steps: Vec<WorkJournalNextStepArtifact>,
	#[serde(default)]
	pub(crate) inferred_next_steps: Vec<WorkJournalNextStepArtifact>,
	#[serde(default)]
	pub(crate) rejected_options: Vec<WorkJournalRejectedOptionArtifact>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct WorkJournalRedactionAuditArtifact {
	#[serde(default)]
	pub(crate) required_marker_ids: Vec<String>,
	#[serde(default)]
	pub(crate) redacted_marker_ids: Vec<String>,
	#[serde(default)]
	pub(crate) persisted_sensitive_marker_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct WorkJournalNextStepArtifact {
	pub(crate) step_id: String,
	pub(crate) text: String,
	pub(crate) label: String,
	pub(crate) instruction: bool,
	#[serde(default)]
	pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct WorkJournalRejectedOptionArtifact {
	pub(crate) option_id: String,
	pub(crate) text: String,
	#[serde(default)]
	pub(crate) evidence_refs: Vec<String>,
	pub(crate) resurrected_as_current: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct WorkJournalWhereStoppedArtifact {
	#[serde(default)]
	pub(crate) reset_resume_entry_ids: Vec<String>,
	#[serde(default)]
	pub(crate) decision_rationale_evidence_ids: Vec<String>,
	#[serde(default)]
	pub(crate) current_explicit_next_step_ids: Vec<String>,
	#[serde(default)]
	pub(crate) labeled_inferred_next_step_ids: Vec<String>,
	#[serde(default)]
	pub(crate) handoff_source_refs: Vec<String>,
	#[serde(default)]
	pub(crate) journal_only_authority_claims: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct WorkJournalPromotionBoundaryArtifact {
	pub(crate) journal_entry_authority: String,
	pub(crate) memory_promotion_required: bool,
	#[serde(default)]
	pub(crate) accepted_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct WorkJournalJanitorCandidateArtifact {
	pub(crate) candidate_id: String,
	#[serde(default)]
	pub(crate) evidence_refs: Vec<String>,
	pub(crate) review_required: bool,
	pub(crate) promoted_to_memory: bool,
}
