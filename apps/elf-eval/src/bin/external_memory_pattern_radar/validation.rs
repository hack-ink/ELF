use std::{collections::BTreeSet, path::Path};

use color_eyre::{Result, eyre};

use super::{
	CURSOR_SCHEMA, RUN_SCHEMA,
	io::read_cursor,
	types::{ElfVerdict, IssueAction, RadarCursor, RadarDecision, RadarProject, RadarRun},
};

pub(super) fn validate_command(path: &Path) -> Result<()> {
	let cursor = read_cursor(path)?;

	validate_cursor(&cursor)
}

pub(super) fn validate_cursor(cursor: &RadarCursor) -> Result<()> {
	let mut errors = Vec::new();

	if cursor.schema != CURSOR_SCHEMA {
		errors.push(format!("cursor schema must be {CURSOR_SCHEMA}"));
	}
	if cursor.projects.is_empty() {
		errors.push("cursor must include at least one project".to_string());
	}

	let project_ids =
		cursor.projects.iter().map(|project| project.id.as_str()).collect::<BTreeSet<_>>();

	if project_ids.len() != cursor.projects.len() {
		errors.push("project ids must be unique".to_string());
	}

	for project in &cursor.projects {
		validate_project(project, &mut errors);
	}

	if let Some(run) = &cursor.last_run {
		validate_run(run, &project_ids, &mut errors);
	}

	if errors.is_empty() {
		Ok(())
	} else {
		Err(eyre::eyre!("radar cursor validation failed:\n{}", errors.join("\n")))
	}
}

fn validate_project(project: &RadarProject, errors: &mut Vec<String>) {
	if project.id.trim().is_empty() {
		errors.push("project id must not be empty".to_string());
	}
	if !project.repo.contains('/') {
		errors.push(format!("project {} repo must be owner/name", project.id));
	}
	if project.coverage_evidence.is_empty() {
		errors.push(format!("project {} must include duplicate/coverage evidence", project.id));
	}
}

fn validate_run(run: &RadarRun, project_ids: &BTreeSet<&str>, errors: &mut Vec<String>) {
	if run.schema != RUN_SCHEMA {
		errors.push(format!("run schema must be {RUN_SCHEMA}"));
	}
	if run.decisions.len() != project_ids.len() {
		errors.push("latest run must include one decision per project".to_string());
	}

	for decision in &run.decisions {
		validate_decision(decision, project_ids, errors);
	}
}

fn validate_decision(
	decision: &RadarDecision,
	project_ids: &BTreeSet<&str>,
	errors: &mut Vec<String>,
) {
	if !project_ids.contains(decision.project_id.as_str()) {
		errors.push(format!("decision references unknown project {}", decision.project_id));
	}

	for (field, value) in [
		("upstream_change", &decision.upstream_change),
		("reusable_pattern", &decision.reusable_pattern),
		("product_value", &decision.product_value),
		("safety_boundary", &decision.safety_boundary),
	] {
		if value.trim().is_empty() {
			errors.push(format!("decision {} has empty {field}", decision.project_id));
		}
	}

	if decision.duplicate_coverage_evidence.is_empty() {
		errors.push(format!(
			"decision {} must include duplicate/coverage evidence",
			decision.project_id
		));
	}
	if decision.acceptance_evidence.is_empty() {
		errors.push(format!("decision {} must include acceptance evidence", decision.project_id));
	}
	if decision.source_links.is_empty() {
		errors.push(format!("decision {} must include source links", decision.project_id));
	}

	validate_issue_decision(decision, errors);
}

fn validate_issue_decision(decision: &RadarDecision, errors: &mut Vec<String>) {
	let issue_decision = &decision.issue_decision;

	if issue_decision.rationale.trim().is_empty() {
		errors.push(format!("decision {} issue rationale must not be empty", decision.project_id));
	}

	match issue_decision.action {
		IssueAction::CreateIssue => validate_create_issue(decision, errors),
		IssueAction::NoIssue =>
			if issue_decision.proposed_issue.is_some() {
				errors.push(format!(
					"decision {} must not include proposed_issue for no_issue",
					decision.project_id
				));
			},
		IssueAction::Defer => {},
	}
}

fn validate_create_issue(decision: &RadarDecision, errors: &mut Vec<String>) {
	let issue_decision = &decision.issue_decision;

	if decision.elf_verdict != ElfVerdict::Gap {
		errors.push(format!(
			"decision {} can create issues only for gap verdicts",
			decision.project_id
		));
	}
	if !issue_decision.duplicate_search.queried {
		errors.push(format!(
			"decision {} must search Linear before issue creation",
			decision.project_id
		));
	}

	let Some(proposed_issue) = &issue_decision.proposed_issue else {
		errors.push(format!(
			"decision {} create_issue must include proposed_issue",
			decision.project_id
		));

		return;
	};

	if proposed_issue.source_links.is_empty()
		|| proposed_issue.repo_evidence.is_empty()
		|| proposed_issue.non_goals.is_empty()
		|| proposed_issue.validation_criteria.is_empty()
	{
		errors.push(format!(
			"decision {} proposed issue must include source links, repo evidence, non-goals, and validation criteria",
			decision.project_id
		));
	}
}
