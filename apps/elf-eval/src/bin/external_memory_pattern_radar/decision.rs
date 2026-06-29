use std::collections::BTreeSet;

use crate::types::{
	DuplicateSearchEvidence, DuplicateSearchResult, ElfVerdict, IssueAction, IssueDecision,
	ProjectObservation, RadarDecision, RadarMode, RadarProject, RunSummary,
};

pub(super) fn decide_project(
	project: &RadarProject,
	prior: Option<&ProjectObservation>,
	observed: &ProjectObservation,
	mode: RadarMode,
) -> RadarDecision {
	let source_links = source_links(project, observed);
	let evidence = project.coverage_evidence.clone();
	let changed = prior.map(|previous| observation_changed(previous, observed)).unwrap_or(false);

	if changed {
		return RadarDecision {
			project_id: project.id.clone(),
			upstream_change: metadata_delta(prior, observed),
			reusable_pattern: "No reusable pattern is claimed from metadata alone; source review is required before a pattern can become a gap."
				.to_string(),
			elf_verdict: ElfVerdict::Reject,
			product_value: "Metadata movement is useful as a review trigger, but it has no product value until source evidence identifies a reusable pattern."
				.to_string(),
			duplicate_coverage_evidence: evidence,
			safety_boundary: "Reject issue creation from activity, star counts, release tags, or push timestamps alone."
				.to_string(),
			issue_decision: IssueDecision {
				action: IssueAction::NoIssue,
				rationale: "No issue was created because this run only proved a metadata delta; the Codex review step must gather source links, repo evidence, and Linear duplicate search first."
					.to_string(),
				duplicate_search: DuplicateSearchEvidence {
					queried: false,
					query: String::new(),
					result: DuplicateSearchResult::NotRequiredNoIssue,
					evidence: vec![
						"No Linear search is required when the issue decision is no_issue.".to_string(),
					],
				},
				proposed_issue: None,
			},
			acceptance_evidence: vec![
				"Metadata delta recorded in the structured cursor.".to_string(),
				"No parity or adoption claim was made from activity alone.".to_string(),
			],
			source_links,
		};
	}

	let upstream_change = if prior.is_none() {
		metadata_delta(None, observed)
	} else {
		match mode {
			RadarMode::Live =>
				"No GitHub metadata delta was observed since the prior cursor.".to_string(),
			RadarMode::Offline =>
				"No upstream fetch was performed; the dry run replayed the checked-in cursor."
					.to_string(),
		}
	};

	RadarDecision {
		project_id: project.id.clone(),
		upstream_change,
		reusable_pattern: "No new candidate pattern was identified in this run.".to_string(),
		elf_verdict: ElfVerdict::Covered,
		product_value: "Current ELF coverage remains represented by the comparison and inventory evidence."
			.to_string(),
		duplicate_coverage_evidence: evidence,
		safety_boundary: "No external runtime is adopted by default; existing ELF evidence remains authoritative."
			.to_string(),
		issue_decision: IssueDecision {
			action: IssueAction::NoIssue,
			rationale: "No issue was created because the run found no source-backed gap.".to_string(),
			duplicate_search: DuplicateSearchEvidence {
				queried: false,
				query: String::new(),
				result: DuplicateSearchResult::NotRequiredNoIssue,
				evidence: vec![
					"No Linear search is required when the issue decision is no_issue.".to_string(),
				],
			},
			proposed_issue: None,
		},
		acceptance_evidence: vec![
			"No-issue decision recorded in the cursor.".to_string(),
			"Coverage evidence points at checked-in ELF research docs.".to_string(),
		],
		source_links,
	}
}

pub(super) fn summarize_decisions(decisions: &[RadarDecision]) -> RunSummary {
	let mut summary = RunSummary { project_count: decisions.len(), ..RunSummary::default() };

	for decision in decisions {
		match decision.elf_verdict {
			ElfVerdict::Covered => summary.covered_count += 1,
			ElfVerdict::Reject => summary.rejected_count += 1,
			ElfVerdict::Gap => summary.gap_count += 1,
		}
		match decision.issue_decision.action {
			IssueAction::NoIssue => summary.no_issue_count += 1,
			IssueAction::Defer => summary.defer_count += 1,
			IssueAction::CreateIssue => summary.create_issue_count += 1,
		}
	}

	summary
}

fn source_links(project: &RadarProject, observed: &ProjectObservation) -> Vec<String> {
	let mut links = BTreeSet::new();

	links.insert(project.homepage.clone());
	links.insert(observed.source_url.clone());

	if let Some(release) = &observed.latest_release {
		links.insert(release.url.clone());
	}

	links.into_iter().collect()
}

fn observation_changed(previous: &ProjectObservation, observed: &ProjectObservation) -> bool {
	previous.pushed_at != observed.pushed_at
		|| previous.updated_at != observed.updated_at
		|| previous.latest_release.as_ref().map(|release| &release.tag_name)
			!= observed.latest_release.as_ref().map(|release| &release.tag_name)
}

fn metadata_delta(prior: Option<&ProjectObservation>, observed: &ProjectObservation) -> String {
	let Some(previous) = prior else {
		return "First cursor observation recorded; no prior state exists for comparison."
			.to_string();
	};
	let previous_release =
		previous.latest_release.as_ref().map(|release| release.tag_name.as_str()).unwrap_or("none");
	let observed_release =
		observed.latest_release.as_ref().map(|release| release.tag_name.as_str()).unwrap_or("none");

	format!(
		"Repository metadata changed: pushed_at {} -> {}, latest_release {} -> {}.",
		previous.pushed_at.as_deref().unwrap_or("unknown"),
		observed.pushed_at.as_deref().unwrap_or("unknown"),
		previous_release,
		observed_release
	)
}
