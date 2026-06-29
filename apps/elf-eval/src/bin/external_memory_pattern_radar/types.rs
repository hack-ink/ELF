use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct RadarCursor {
	pub(super) schema: String,
	pub(super) cadence: String,
	pub(super) generated_at: String,
	pub(super) source_docs: Vec<String>,
	pub(super) projects: Vec<RadarProject>,
	pub(super) last_run: Option<RadarRun>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct RadarProject {
	pub(super) id: String,
	pub(super) name: String,
	pub(super) repo: String,
	pub(super) homepage: String,
	pub(super) watch_focus: Vec<String>,
	pub(super) primary_references: Vec<String>,
	pub(super) coverage_evidence: Vec<EvidenceRef>,
	pub(super) last_seen: Option<ProjectObservation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct EvidenceRef {
	pub(super) label: String,
	pub(super) path: String,
	pub(super) summary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct ProjectObservation {
	pub(super) observed_at: String,
	pub(super) source_url: String,
	pub(super) default_branch: Option<String>,
	pub(super) pushed_at: Option<String>,
	pub(super) updated_at: Option<String>,
	pub(super) latest_release: Option<ReleaseObservation>,
	pub(super) stars: Option<u64>,
	pub(super) open_issues: Option<u64>,
	pub(super) description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct ReleaseObservation {
	pub(super) tag_name: String,
	pub(super) url: String,
	pub(super) published_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct RadarRun {
	pub(super) schema: String,
	pub(super) run_id: String,
	pub(super) generated_at: String,
	pub(super) mode: RadarMode,
	pub(super) summary: RunSummary,
	pub(super) decisions: Vec<RadarDecision>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct RunSummary {
	pub(super) project_count: usize,
	pub(super) covered_count: usize,
	pub(super) rejected_count: usize,
	pub(super) gap_count: usize,
	pub(super) create_issue_count: usize,
	pub(super) defer_count: usize,
	pub(super) no_issue_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct RadarDecision {
	pub(super) project_id: String,
	pub(super) upstream_change: String,
	pub(super) reusable_pattern: String,
	pub(super) elf_verdict: ElfVerdict,
	pub(super) product_value: String,
	pub(super) duplicate_coverage_evidence: Vec<EvidenceRef>,
	pub(super) safety_boundary: String,
	pub(super) issue_decision: IssueDecision,
	pub(super) acceptance_evidence: Vec<String>,
	pub(super) source_links: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct IssueDecision {
	pub(super) action: IssueAction,
	pub(super) rationale: String,
	pub(super) duplicate_search: DuplicateSearchEvidence,
	pub(super) proposed_issue: Option<ProposedIssue>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct DuplicateSearchEvidence {
	pub(super) queried: bool,
	pub(super) query: String,
	pub(super) result: DuplicateSearchResult,
	pub(super) evidence: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct ProposedIssue {
	pub(super) title: String,
	pub(super) source_links: Vec<String>,
	pub(super) repo_evidence: Vec<String>,
	pub(super) non_goals: Vec<String>,
	pub(super) validation_criteria: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub(super) enum RadarMode {
	Live,
	Offline,
}
impl RadarMode {
	pub(super) fn as_str(self) -> &'static str {
		match self {
			Self::Live => "live",
			Self::Offline => "offline",
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ElfVerdict {
	Covered,
	Reject,
	Gap,
}
impl ElfVerdict {
	pub(super) fn as_str(self) -> &'static str {
		match self {
			Self::Covered => "covered",
			Self::Reject => "reject",
			Self::Gap => "gap",
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum IssueAction {
	NoIssue,
	Defer,
	CreateIssue,
}
impl IssueAction {
	pub(super) fn as_str(self) -> &'static str {
		match self {
			Self::NoIssue => "no_issue",
			Self::Defer => "defer",
			Self::CreateIssue => "create_issue",
		}
	}
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum DuplicateSearchResult {
	NotRequiredNoIssue,
	NoDuplicateFound,
	DuplicateFound,
}
