#![allow(unused_crate_dependencies)]

//! Weekly external memory pattern radar runner.

use std::{
	collections::BTreeSet,
	env, fs,
	path::{Path, PathBuf},
};

use clap::{Parser, Subcommand, ValueEnum};
use color_eyre::{Result, eyre};
use reqwest::{
	Client, StatusCode,
	header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT},
};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const CURSOR_SCHEMA: &str = "elf.external_memory_pattern_radar_cursor/v1";
const RUN_SCHEMA: &str = "elf.external_memory_pattern_radar_run/v1";
const DEFAULT_CURSOR: &str = "apps/elf-eval/fixtures/external_memory_pattern_radar/cursor.json";
const DEFAULT_SUMMARY: &str = "docs/evidence/external_memory_pattern_radar_latest.md";

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
struct Args {
	#[command(subcommand)]
	command: Command,
}

#[derive(Debug, Parser)]
struct RunArgs {
	/// Existing radar cursor file.
	#[arg(long, value_name = "FILE", default_value = DEFAULT_CURSOR)]
	cursor: PathBuf,
	/// Output cursor path. Defaults to updating --cursor.
	#[arg(long, value_name = "FILE")]
	out_cursor: Option<PathBuf>,
	/// Output Markdown summary path.
	#[arg(long, value_name = "FILE", default_value = DEFAULT_SUMMARY)]
	summary: PathBuf,
	/// Observation mode. Use offline for deterministic dry runs.
	#[arg(long, value_enum, default_value_t = RadarMode::Live)]
	mode: RadarMode,
	/// Stable run id. Defaults to external-memory-pattern-radar-YYYY-MM-DD.
	#[arg(long)]
	run_id: Option<String>,
	/// Environment variable containing a GitHub token for live mode.
	#[arg(long, default_value = "GITHUB_TOKEN")]
	github_token_env: String,
}

#[derive(Debug, Parser)]
struct ValidateArgs {
	/// Cursor file to validate.
	#[arg(long, value_name = "FILE", default_value = DEFAULT_CURSOR)]
	cursor: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct RadarCursor {
	schema: String,
	cadence: String,
	generated_at: String,
	source_docs: Vec<String>,
	projects: Vec<RadarProject>,
	last_run: Option<RadarRun>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct RadarProject {
	id: String,
	name: String,
	repo: String,
	homepage: String,
	watch_focus: Vec<String>,
	primary_references: Vec<String>,
	coverage_evidence: Vec<EvidenceRef>,
	last_seen: Option<ProjectObservation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct EvidenceRef {
	label: String,
	path: String,
	summary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct ProjectObservation {
	observed_at: String,
	source_url: String,
	default_branch: Option<String>,
	pushed_at: Option<String>,
	updated_at: Option<String>,
	latest_release: Option<ReleaseObservation>,
	stars: Option<u64>,
	open_issues: Option<u64>,
	description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct ReleaseObservation {
	tag_name: String,
	url: String,
	published_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct RadarRun {
	schema: String,
	run_id: String,
	generated_at: String,
	mode: RadarMode,
	summary: RunSummary,
	decisions: Vec<RadarDecision>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct RunSummary {
	project_count: usize,
	covered_count: usize,
	rejected_count: usize,
	gap_count: usize,
	create_issue_count: usize,
	defer_count: usize,
	no_issue_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct RadarDecision {
	project_id: String,
	upstream_change: String,
	reusable_pattern: String,
	elf_verdict: ElfVerdict,
	product_value: String,
	duplicate_coverage_evidence: Vec<EvidenceRef>,
	safety_boundary: String,
	issue_decision: IssueDecision,
	acceptance_evidence: Vec<String>,
	source_links: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct IssueDecision {
	action: IssueAction,
	rationale: String,
	duplicate_search: DuplicateSearchEvidence,
	proposed_issue: Option<ProposedIssue>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct DuplicateSearchEvidence {
	queried: bool,
	query: String,
	result: DuplicateSearchResult,
	evidence: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct ProposedIssue {
	title: String,
	source_links: Vec<String>,
	repo_evidence: Vec<String>,
	non_goals: Vec<String>,
	validation_criteria: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GithubRepoResponse {
	html_url: String,
	default_branch: Option<String>,
	pushed_at: Option<String>,
	updated_at: Option<String>,
	stargazers_count: Option<u64>,
	open_issues_count: Option<u64>,
	description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubReleaseResponse {
	tag_name: String,
	html_url: String,
	published_at: Option<String>,
}

#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab")]
enum Command {
	/// Run the external memory radar and write cursor plus Markdown summary.
	Run(RunArgs),
	/// Validate a radar cursor and its latest decision records.
	Validate(ValidateArgs),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum RadarMode {
	Live,
	Offline,
}
impl RadarMode {
	fn as_str(self) -> &'static str {
		match self {
			Self::Live => "live",
			Self::Offline => "offline",
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ElfVerdict {
	Covered,
	Reject,
	Gap,
}
impl ElfVerdict {
	fn as_str(self) -> &'static str {
		match self {
			Self::Covered => "covered",
			Self::Reject => "reject",
			Self::Gap => "gap",
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum IssueAction {
	NoIssue,
	Defer,
	CreateIssue,
}
impl IssueAction {
	fn as_str(self) -> &'static str {
		match self {
			Self::NoIssue => "no_issue",
			Self::Defer => "defer",
			Self::CreateIssue => "create_issue",
		}
	}
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum DuplicateSearchResult {
	NotRequiredNoIssue,
	NoDuplicateFound,
	DuplicateFound,
}

fn validate_command(path: &Path) -> Result<()> {
	let cursor = read_cursor(path)?;

	validate_cursor(&cursor)
}

fn read_cursor(path: &Path) -> Result<RadarCursor> {
	let raw = fs::read_to_string(path)
		.map_err(|err| eyre::eyre!("failed to read cursor {}: {err}", path.display()))?;
	let cursor = serde_json::from_str(&raw)
		.map_err(|err| eyre::eyre!("failed to parse cursor {}: {err}", path.display()))?;

	Ok(cursor)
}

fn write_json<T>(path: &Path, value: &T) -> Result<()>
where
	T: Serialize,
{
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)?;
	}

	let raw = serde_json::to_string_pretty(value)?;

	fs::write(path, format!("{raw}\n"))?;

	Ok(())
}

fn write_text(path: &Path, content: &str) -> Result<()> {
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)?;
	}

	fs::write(path, content)?;

	Ok(())
}

fn github_client(token_env: &str) -> Result<Option<Client>> {
	let mut headers = HeaderMap::new();

	headers.insert(USER_AGENT, HeaderValue::from_static("elf-external-memory-pattern-radar"));
	headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github+json"));

	if let Ok(token) = env::var(token_env)
		&& !token.trim().is_empty()
	{
		let value = format!("Bearer {}", token.trim()).parse()?;

		headers.insert(AUTHORIZATION, value);
	}

	Ok(Some(Client::builder().default_headers(headers).build()?))
}

fn fallback_observation(project: &RadarProject, generated_at: &str) -> ProjectObservation {
	ProjectObservation {
		observed_at: generated_at.to_string(),
		source_url: project.homepage.clone(),
		default_branch: None,
		pushed_at: None,
		updated_at: None,
		latest_release: None,
		stars: None,
		open_issues: None,
		description: None,
	}
}

fn decide_project(
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

fn summarize_decisions(decisions: &[RadarDecision]) -> RunSummary {
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

fn validate_cursor(cursor: &RadarCursor) -> Result<()> {
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

fn render_summary(cursor: &RadarCursor) -> Result<String> {
	let run = cursor.last_run.as_ref().ok_or_else(|| eyre::eyre!("cursor has no last_run"))?;
	let last_verified = run.generated_at.get(..10).unwrap_or("unknown");
	let mut out = String::new();

	out.push_str("---\n");
	out.push_str("type: Evidence\n");
	out.push_str("title: \"External Memory Pattern Radar Summary\"\n");
	out.push_str("description: \"Latest weekly ELF external memory pattern radar outcome.\"\n");
	out.push_str("resource: docs/evidence/external_memory_pattern_radar_latest.md\n");
	out.push_str("status: active\n");
	out.push_str("authority: current_state\n");
	out.push_str("owner: evidence\n");
	out.push_str(&format!("last_verified: {last_verified}\n"));
	out.push_str("tags:\n");
	out.push_str("  - docs\n");
	out.push_str("  - external-memory-pattern-radar\n");
	out.push_str("  - evidence\n");
	out.push_str("source_refs: []\n");
	out.push_str("code_refs:\n");
	out.push_str("  - apps/elf-eval/fixtures/external_memory_pattern_radar/cursor.json\n");
	out.push_str("  - apps/elf-eval/src/bin/external_memory_pattern_radar.rs\n");
	out.push_str("related: []\n");
	out.push_str("drift_watch:\n");
	out.push_str("  - apps/elf-eval/fixtures/external_memory_pattern_radar/cursor.json\n");
	out.push_str("  - apps/elf-eval/src/bin/external_memory_pattern_radar.rs\n");
	out.push_str("---\n\n");
	out.push_str("# External Memory Pattern Radar Summary\n\n");
	out.push_str("Goal: Preserve the latest weekly ELF external memory pattern radar outcome.\n");
	out.push_str("Read this when: Feeding the next full comparison report or deciding whether a watched upstream memory project created an ELF follow-up.\n");
	out.push_str("Inputs: `apps/elf-eval/fixtures/external_memory_pattern_radar/cursor.json`, GitHub repository metadata, checked-in ELF comparison evidence, and any Codex source-review notes.\n");
	out.push_str("Depends on: `docs/spec/external_memory_pattern_radar_v1.md` and `docs/runbook/external_memory_pattern_radar.md`.\n");
	out.push_str("Outputs: Latest no-issue, rejection, or issue-ready radar decisions.\n\n");
	out.push_str(&format!("- Run id: `{}`\n", run.run_id));
	out.push_str(&format!("- Generated at: `{}`\n", run.generated_at));
	out.push_str(&format!("- Mode: `{}`\n", run.mode.as_str()));
	out.push_str(&format!(
		"- Projects: `{}`; covered: `{}`; rejected: `{}`; gaps: `{}`; create_issue: `{}`\n\n",
		run.summary.project_count,
		run.summary.covered_count,
		run.summary.rejected_count,
		run.summary.gap_count,
		run.summary.create_issue_count
	));
	out.push_str("## Decisions\n\n");
	out.push_str(
		"| Project | Upstream change | ELF verdict | Issue decision | Acceptance evidence |\n",
	);
	out.push_str("| --- | --- | --- | --- | --- |\n");

	for decision in &run.decisions {
		out.push_str(&format!(
			"| `{}` | {} | `{}` | `{}` | {} |\n",
			decision.project_id,
			escape_markdown_table(&decision.upstream_change),
			decision.elf_verdict.as_str(),
			decision.issue_decision.action.as_str(),
			escape_markdown_table(&decision.acceptance_evidence.join("; "))
		));
	}

	out.push_str("\n## Safety Boundary\n\n");
	out.push_str("- The radar records upstream movement as a trigger for source review, not as proof of parity or a reason to adopt an external runtime.\n");
	out.push_str("- `create_issue` decisions are valid only when the cursor includes source links, repo evidence, non-goals, validation criteria, and Linear duplicate-search evidence.\n");
	out.push_str("- No-issue runs remain useful because each project records why ELF is already covered or why metadata-only movement was rejected.\n");

	Ok(out)
}

fn escape_markdown_table(value: &str) -> String {
	value.replace('|', "\\|").replace('\n', " ")
}

fn format_rfc3339(value: OffsetDateTime) -> Result<String> {
	Ok(value.format(&Rfc3339)?)
}

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;

	match Args::parse().command {
		Command::Run(args) => run_radar(args).await,
		Command::Validate(args) => validate_command(&args.cursor),
	}
}

async fn run_radar(args: RunArgs) -> Result<()> {
	let now = OffsetDateTime::now_utc();
	let generated_at = format_rfc3339(now)?;
	let run_id =
		args.run_id.unwrap_or_else(|| format!("external-memory-pattern-radar-{}", now.date()));
	let client = github_client(&args.github_token_env)?;
	let mut cursor = read_cursor(&args.cursor)?;
	let mut decisions = Vec::with_capacity(cursor.projects.len());

	for project in &mut cursor.projects {
		let prior = project.last_seen.clone();
		let observed = observe_project(project, args.mode, client.as_ref(), &generated_at).await?;

		decisions.push(decide_project(project, prior.as_ref(), &observed, args.mode));

		project.last_seen = Some(observed);
	}

	let summary = summarize_decisions(&decisions);

	cursor.generated_at = generated_at.clone();
	cursor.last_run = Some(RadarRun {
		schema: RUN_SCHEMA.to_string(),
		run_id,
		generated_at,
		mode: args.mode,
		summary,
		decisions,
	});

	validate_cursor(&cursor)?;

	let out_cursor = args.out_cursor.unwrap_or(args.cursor);

	write_json(&out_cursor, &cursor)?;
	write_text(&args.summary, &render_summary(&cursor)?)?;

	Ok(())
}

async fn observe_project(
	project: &RadarProject,
	mode: RadarMode,
	client: Option<&Client>,
	generated_at: &str,
) -> Result<ProjectObservation> {
	match mode {
		RadarMode::Offline => Ok(project
			.last_seen
			.clone()
			.unwrap_or_else(|| fallback_observation(project, generated_at))),
		RadarMode::Live =>
			fetch_project(
				project,
				client.ok_or_else(|| eyre::eyre!("missing GitHub client"))?,
				generated_at,
			)
			.await,
	}
}

async fn fetch_project(
	project: &RadarProject,
	client: &Client,
	generated_at: &str,
) -> Result<ProjectObservation> {
	let repo = fetch_repo(project, client).await?;
	let latest_release = fetch_latest_release(project, client).await?;

	Ok(ProjectObservation {
		observed_at: generated_at.to_string(),
		source_url: repo.html_url,
		default_branch: repo.default_branch,
		pushed_at: repo.pushed_at,
		updated_at: repo.updated_at,
		latest_release,
		stars: repo.stargazers_count,
		open_issues: repo.open_issues_count,
		description: repo.description,
	})
}

async fn fetch_repo(project: &RadarProject, client: &Client) -> Result<GithubRepoResponse> {
	let url = format!("https://api.github.com/repos/{}", project.repo);
	let response = client.get(url).send().await?;

	if !response.status().is_success() {
		return Err(eyre::eyre!(
			"GitHub repo metadata fetch failed for {} with status {}",
			project.repo,
			response.status()
		));
	}

	Ok(response.json().await?)
}

async fn fetch_latest_release(
	project: &RadarProject,
	client: &Client,
) -> Result<Option<ReleaseObservation>> {
	let url = format!("https://api.github.com/repos/{}/releases/latest", project.repo);
	let response = client.get(url).send().await?;

	if response.status() == StatusCode::NOT_FOUND {
		return Ok(None);
	}
	if !response.status().is_success() {
		return Err(eyre::eyre!(
			"GitHub release metadata fetch failed for {} with status {}",
			project.repo,
			response.status()
		));
	}

	let release: GithubReleaseResponse = response.json().await?;

	Ok(Some(ReleaseObservation {
		tag_name: release.tag_name,
		url: release.html_url,
		published_at: release.published_at,
	}))
}
