#![allow(clippy::single_component_path_imports, unused_crate_dependencies)]

//! Offline runner and publisher for real-world job benchmark fixtures.

use std::{
	collections::{BTreeMap, BTreeSet},
	fs,
	path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use color_eyre::{Result, eyre};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use elf_cli::VERSION;

const JOB_SCHEMA: &str = "elf.real_world_job/v1";
const REPORT_SCHEMA: &str = "elf.real_world_job_report/v1";
const DEFAULT_FIXTURE_PATH: &str = "apps/elf-eval/fixtures/real_world_job/smoke";
const DEFAULT_REPORT_PATH: &str = "tmp/real-world-job/real-world-job-smoke-report.json";
const DEFAULT_MARKDOWN_PATH: &str = "tmp/real-world-job/real-world-job-smoke-report.md";
const DEFAULT_RUN_ID: &str = "real-world-job-smoke";
const DEFAULT_ADAPTER_ID: &str = "fixture_smoke";
const DEFAULT_ADAPTER_NAME: &str = "ELF fixture smoke";
const NOT_ENCODED_REASON: &str = "No checked-in real_world_job fixture is encoded for this suite.";
const SUITES: &[&str] = &[
	"trust_source_of_truth",
	"work_resume",
	"project_decisions",
	"retrieval",
	"memory_evolution",
	"consolidation",
	"knowledge_compilation",
	"operator_debugging_ux",
	"capture_integration",
	"production_ops",
	"personalization",
];

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

#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab")]
enum Command {
	/// Parse and score real_world_job fixtures, then emit a JSON report.
	Run(RunArgs),
	/// Render Markdown from a generated real_world_job JSON report.
	Publish(PublishArgs),
}

#[derive(Debug, Parser)]
struct RunArgs {
	/// Fixture file or directory containing real_world_job JSON fixtures.
	#[arg(long, value_name = "PATH", default_value = DEFAULT_FIXTURE_PATH)]
	fixtures: PathBuf,
	/// Write report JSON to this file. Omit to print to stdout.
	#[arg(long, value_name = "FILE")]
	out: Option<PathBuf>,
	/// Stable run id recorded in the generated report.
	#[arg(long, default_value = DEFAULT_RUN_ID)]
	run_id: String,
	/// Adapter id recorded for the offline smoke response.
	#[arg(long, default_value = DEFAULT_ADAPTER_ID)]
	adapter_id: String,
	/// Human-readable adapter name recorded in the generated report.
	#[arg(long, default_value = DEFAULT_ADAPTER_NAME)]
	adapter_name: String,
}

#[derive(Debug, Parser)]
struct PublishArgs {
	/// Generated real_world_job JSON report.
	#[arg(long, value_name = "FILE", default_value = DEFAULT_REPORT_PATH)]
	report: PathBuf,
	/// Write Markdown to this file. Omit to print to stdout.
	#[arg(long, value_name = "FILE", default_value = DEFAULT_MARKDOWN_PATH)]
	out: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct RealWorldJob {
	schema: String,
	job_id: String,
	suite: String,
	title: String,
	corpus: Corpus,
	#[serde(default)]
	timeline: Vec<TimelineEvent>,
	prompt: Prompt,
	expected_answer: ExpectedAnswer,
	#[serde(default)]
	required_evidence: Vec<RequiredEvidence>,
	#[serde(default)]
	negative_traps: Vec<NegativeTrap>,
	scoring_rubric: ScoringRubric,
	allowed_uncertainty: AllowedUncertainty,
	#[serde(default)]
	tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Corpus {
	corpus_id: String,
	profile: CorpusProfile,
	#[serde(default)]
	items: Vec<CorpusItem>,

	adapter_response: Option<AdapterResponse>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum CorpusProfile {
	Synthetic,
	PrivateSanitized,
	GeneratedPublic,
	ExternalAdapter,
}
impl CorpusProfile {
	fn as_str(&self) -> &'static str {
		match self {
			Self::Synthetic => "synthetic",
			Self::PrivateSanitized => "private_sanitized",
			Self::GeneratedPublic => "generated_public",
			Self::ExternalAdapter => "external_adapter",
		}
	}
}

#[derive(Debug, Deserialize)]
struct CorpusItem {
	evidence_id: String,
	kind: String,

	text: Option<String>,

	local_ref: Option<String>,
	#[serde(default)]
	source_ref: Value,

	created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TimelineEvent {
	event_id: String,
	ts: String,
	actor: String,
	action: String,
	#[serde(default)]
	evidence_ids: Vec<String>,
	summary: String,
}

#[derive(Debug, Deserialize)]
struct Prompt {
	role: String,
	content: String,
	job_mode: String,
	#[serde(default)]
	constraints: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedAnswer {
	#[serde(default)]
	must_include: Vec<ExpectedClaim>,
	#[serde(default)]
	must_not_include: Vec<String>,
	#[serde(default)]
	evidence_links: BTreeMap<String, EvidenceLink>,
	answer_type: String,
	#[serde(default)]
	accepted_alternates: Vec<Value>,
	#[serde(default)]
	requires_caveat: bool,
	#[serde(default)]
	requires_refusal: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum ExpectedClaim {
	Text(String),
	Object { claim_id: Option<String>, text: String },
}
impl ExpectedClaim {
	fn claim_id(&self) -> Option<&str> {
		match self {
			Self::Text(_) => None,
			Self::Object { claim_id, .. } => claim_id.as_deref(),
		}
	}

	fn text(&self) -> &str {
		match self {
			Self::Text(text) => text,
			Self::Object { text, .. } => text,
		}
	}
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum EvidenceLink {
	One(String),
	Many(Vec<String>),
}
impl EvidenceLink {
	fn ids(&self) -> BTreeSet<String> {
		match self {
			Self::One(id) => BTreeSet::from([id.clone()]),
			Self::Many(ids) => ids.iter().cloned().collect(),
		}
	}
}

#[derive(Debug, Deserialize)]
struct RequiredEvidence {
	evidence_id: String,
	claim_id: String,
	requirement: String,

	quote: Option<String>,

	selector: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NegativeTrap {
	trap_id: String,
	#[serde(rename = "type")]
	trap_type: String,
	#[serde(default)]
	evidence_ids: Vec<String>,
	#[serde(default)]
	failure_if_used: bool,
}

#[derive(Debug, Deserialize)]
struct ScoringRubric {
	#[serde(default)]
	dimensions: BTreeMap<String, RubricDimension>,
	pass_threshold: f64,
	#[serde(default)]
	hard_fail_rules: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RubricDimension {
	weight: f64,
	max_points: f64,
	criteria: Value,
}

#[derive(Debug, Deserialize)]
struct AllowedUncertainty {
	can_answer_unknown: bool,
	#[serde(default)]
	acceptable_phrases: Vec<String>,
	fallback_action: String,
}

#[derive(Clone, Debug, Deserialize)]
struct AdapterResponse {
	adapter_id: Option<String>,
	answer: ProducedAnswer,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ProducedAnswer {
	content: String,
	#[serde(default)]
	claims: Vec<ProducedClaim>,
	#[serde(default)]
	evidence_ids: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	latency_ms: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	cost: Option<CostReport>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ProducedClaim {
	#[serde(skip_serializing_if = "Option::is_none")]
	claim_id: Option<String>,
	text: String,
	#[serde(default)]
	evidence_ids: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	confidence: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CostReport {
	#[serde(skip_serializing_if = "Option::is_none")]
	currency: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	amount: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	input_tokens: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	output_tokens: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum TypedStatus {
	Pass,
	WrongResult,
	LifecycleFail,
	Incomplete,
	Blocked,
	NotEncoded,
	UnsupportedClaim,
}

#[derive(Debug, Deserialize, Serialize)]
struct RealWorldReport {
	schema: String,
	run_id: String,
	generated_at: String,
	runner_version: String,
	corpus_profile: String,
	adapter: AdapterReport,
	summary: ReportSummary,
	suites: Vec<SuiteReport>,
	jobs: Vec<JobReport>,
	unsupported_claims: Vec<UnsupportedClaimReport>,
	not_encoded_suites: Vec<String>,
	private_corpus_redaction: PrivateCorpusRedaction,
}

#[derive(Debug, Deserialize, Serialize)]
struct AdapterReport {
	adapter_id: String,
	name: String,
	behavior: String,
	storage: TypedStatus,
	runtime: TypedStatus,
	notes: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct ReportSummary {
	job_count: usize,
	encoded_suite_count: usize,
	pass: usize,
	wrong_result: usize,
	lifecycle_fail: usize,
	incomplete: usize,
	blocked: usize,
	not_encoded: usize,
	unsupported_claim: usize,
	unsupported_claim_count: usize,
	wrong_result_count: usize,
	mean_score: f64,
	mean_latency_ms: Option<f64>,
	total_cost: Option<CostReport>,
	#[serde(default)]
	evidence_required_count: usize,
	#[serde(default)]
	evidence_covered_count: usize,
	#[serde(default)]
	evidence_coverage: f64,
	#[serde(default)]
	source_ref_required_count: usize,
	#[serde(default)]
	source_ref_covered_count: usize,
	#[serde(default)]
	source_ref_coverage: f64,
	#[serde(default)]
	quote_required_count: usize,
	#[serde(default)]
	quote_covered_count: usize,
	#[serde(default)]
	quote_coverage: f64,
	#[serde(default)]
	stale_retrieval_count: usize,
	#[serde(default)]
	scope_check_count: usize,
	#[serde(default)]
	scope_correct_count: usize,
	#[serde(default)]
	scope_correctness: f64,
	#[serde(default)]
	scope_violation_count: usize,
	#[serde(default)]
	redaction_leak_count: usize,
	#[serde(default)]
	qdrant_rebuild_case_count: usize,
	#[serde(default)]
	qdrant_rebuild_pass_count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
struct SuiteReport {
	suite_id: String,
	status: TypedStatus,
	encoded_job_count: usize,
	score_mean: Option<f64>,
	unsupported_claim_count: usize,
	wrong_result_count: usize,
	reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct JobReport {
	suite_id: String,
	job_id: String,
	title: String,
	status: TypedStatus,
	normalized_score: f64,
	hard_fail_hits: Vec<String>,
	expected_evidence: Vec<ExpectedEvidenceReport>,
	produced_answer: String,
	produced_evidence: Vec<String>,
	unsupported_claim_count: usize,
	wrong_result_count: usize,
	latency_ms: Option<f64>,
	cost: Option<CostReport>,
	trap_ids_used: Vec<String>,
	dimension_scores: Vec<DimensionScoreReport>,
	reason: String,
	#[serde(default)]
	evidence_required_count: usize,
	#[serde(default)]
	evidence_covered_count: usize,
	#[serde(default)]
	source_ref_required_count: usize,
	#[serde(default)]
	source_ref_covered_count: usize,
	#[serde(default)]
	quote_required_count: usize,
	#[serde(default)]
	quote_covered_count: usize,
	#[serde(default)]
	stale_retrieval_count: usize,
	#[serde(default)]
	scope_check_count: usize,
	#[serde(default)]
	scope_correct_count: usize,
	#[serde(default)]
	scope_violation_count: usize,
	#[serde(default)]
	redaction_leak_count: usize,
	#[serde(default)]
	qdrant_rebuild_case: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct ExpectedEvidenceReport {
	evidence_id: String,
	claim_id: String,
	requirement: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct DimensionScoreReport {
	dimension: String,
	score: f64,
	max_points: f64,
	weight: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct UnsupportedClaimReport {
	suite_id: String,
	job_id: String,
	claim_id: Option<String>,
	claim_text: String,
	reason: String,
	evidence_ids: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PrivateCorpusRedaction {
	policy: String,
	private_fixture_count: usize,
}

#[derive(Debug)]
struct JobScoring {
	status: TypedStatus,
	normalized_score: f64,
	hard_fail_hits: Vec<String>,
	unsupported_claims: Vec<UnsupportedClaimReport>,
	wrong_result_count: usize,
	trap_ids_used: Vec<String>,
	dimension_scores: Vec<DimensionScoreReport>,
	reason: String,
}

#[derive(Debug, Default)]
struct FailureCounts {
	missing_claims: usize,
	forbidden_claims: usize,
	missing_evidence: usize,
	trap_uses: usize,
	unsupported_claims: usize,
}

#[derive(Debug, Default)]
struct JobMetrics {
	evidence_required_count: usize,
	evidence_covered_count: usize,
	source_ref_required_count: usize,
	source_ref_covered_count: usize,
	quote_required_count: usize,
	quote_covered_count: usize,
	stale_retrieval_count: usize,
	scope_check_count: usize,
	scope_correct_count: usize,
	scope_violation_count: usize,
	redaction_leak_count: usize,
	qdrant_rebuild_case: bool,
}

fn main() -> Result<()> {
	color_eyre::install()?;

	match Args::parse().command {
		Command::Run(args) => run_command(args),
		Command::Publish(args) => publish_command(args),
	}
}

fn run_command(args: RunArgs) -> Result<()> {
	let jobs = load_jobs(&args.fixtures)?;
	let report = build_report(&jobs, &args)?;
	let json = serde_json::to_string_pretty(&report)?;

	write_or_print(args.out.as_deref(), json.as_str())
}

fn publish_command(args: PublishArgs) -> Result<()> {
	let raw = fs::read_to_string(&args.report)?;
	let report = serde_json::from_str::<RealWorldReport>(&raw)?;
	let markdown = render_markdown(&report, &args.report);

	write_or_print(args.out.as_deref(), markdown.as_str())
}

fn load_jobs(path: &Path) -> Result<Vec<RealWorldJob>> {
	let paths = fixture_paths(path)?;
	let mut jobs = Vec::with_capacity(paths.len());

	for fixture in paths {
		let raw = fs::read_to_string(&fixture)?;
		let job = serde_json::from_str::<RealWorldJob>(&raw)
			.map_err(|err| eyre::eyre!("Failed to parse {}: {err}", fixture.display()))?;

		validate_job(&job, &fixture)?;

		jobs.push(job);
	}

	Ok(jobs)
}

fn fixture_paths(path: &Path) -> Result<Vec<PathBuf>> {
	if path.is_file() {
		return Ok(vec![path.to_path_buf()]);
	}
	if !path.is_dir() {
		return Err(eyre::eyre!("Fixture path does not exist: {}", path.display()));
	}

	let mut paths = Vec::new();

	collect_fixture_paths(path, &mut paths)?;

	paths.sort();

	if paths.is_empty() {
		return Err(eyre::eyre!("No JSON fixtures found in {}.", path.display()));
	}

	Ok(paths)
}

fn collect_fixture_paths(path: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
	for entry in fs::read_dir(path)? {
		let entry = entry?;
		let entry_path = entry.path();

		if entry_path.is_dir() {
			collect_fixture_paths(entry_path.as_path(), paths)?;
		} else if entry_path.extension().and_then(|ext| ext.to_str()) == Some("json") {
			paths.push(entry_path);
		}
	}

	Ok(())
}

fn validate_job(job: &RealWorldJob, path: &Path) -> Result<()> {
	if job.schema != JOB_SCHEMA {
		return Err(eyre::eyre!(
			"{} has schema {}, expected {JOB_SCHEMA}.",
			path.display(),
			job.schema
		));
	}

	validate_job_identity(job, path)?;

	if !SUITES.contains(&job.suite.as_str()) {
		return Err(eyre::eyre!("{} uses unknown suite {}.", path.display(), job.suite));
	}

	validate_corpus_items(job, path)?;
	validate_timeline(job, path)?;
	validate_prompt(job, path)?;
	validate_expected_answer(job, path)?;
	validate_required_evidence(job, path)?;
	validate_scoring_rubric(job, path)?;
	validate_allowed_uncertainty(job, path)?;

	Ok(())
}

fn validate_job_identity(job: &RealWorldJob, path: &Path) -> Result<()> {
	if job.job_id.trim().is_empty()
		|| job.suite.trim().is_empty()
		|| job.title.trim().is_empty()
		|| job.corpus.corpus_id.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete job identity.", path.display()));
	}

	for tag in &job.tags {
		if tag.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty tag.", path.display()));
		}
	}

	if let Some(adapter_response) = &job.corpus.adapter_response
		&& adapter_response.adapter_id.as_deref().is_some_and(str::is_empty)
	{
		return Err(eyre::eyre!("{} has an empty adapter_response adapter_id.", path.display()));
	}

	Ok(())
}

fn validate_corpus_items(job: &RealWorldJob, path: &Path) -> Result<()> {
	let mut evidence_ids = BTreeSet::new();

	for item in &job.corpus.items {
		if item.evidence_id.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} has a corpus item with an empty evidence_id.",
				path.display()
			));
		}
		if item.kind.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} has corpus item {} with an empty kind.",
				path.display(),
				item.evidence_id
			));
		}
		if item.text.is_none() && item.local_ref.is_none() {
			return Err(eyre::eyre!(
				"{} corpus item {} must provide text or local_ref.",
				path.display(),
				item.evidence_id
			));
		}
		if !item.source_ref.is_object() {
			return Err(eyre::eyre!(
				"{} corpus item {} must provide an object source_ref.",
				path.display(),
				item.evidence_id
			));
		}

		if let Some(created_at) = &item.created_at {
			validate_optional_rfc3339(created_at, path, item.evidence_id.as_str())?;
		}

		evidence_ids.insert(item.evidence_id.clone());
	}
	for trap in &job.negative_traps {
		if trap.trap_id.trim().is_empty() || trap.trap_type.trim().is_empty() {
			return Err(eyre::eyre!("{} has an incomplete negative trap.", path.display()));
		}

		for evidence_id in &trap.evidence_ids {
			ensure_known_evidence(path, &evidence_ids, evidence_id)?;
		}
	}

	Ok(())
}

fn validate_timeline(job: &RealWorldJob, path: &Path) -> Result<()> {
	let evidence_ids = corpus_evidence_ids(job);

	for event in &job.timeline {
		if event.event_id.trim().is_empty()
			|| event.actor.trim().is_empty()
			|| event.action.trim().is_empty()
			|| event.summary.trim().is_empty()
		{
			return Err(eyre::eyre!("{} has an incomplete timeline event.", path.display()));
		}

		validate_required_rfc3339(event.ts.as_str(), path, event.event_id.as_str())?;

		for evidence_id in &event.evidence_ids {
			ensure_known_evidence(path, &evidence_ids, evidence_id)?;
		}
	}

	Ok(())
}

fn validate_prompt(job: &RealWorldJob, path: &Path) -> Result<()> {
	if job.prompt.role.trim().is_empty()
		|| job.prompt.content.trim().is_empty()
		|| job.prompt.job_mode.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete prompt.", path.display()));
	}

	for constraint in &job.prompt.constraints {
		if constraint.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty prompt constraint.", path.display()));
		}
	}

	Ok(())
}

fn validate_expected_answer(job: &RealWorldJob, path: &Path) -> Result<()> {
	if job.expected_answer.answer_type.trim().is_empty() {
		return Err(eyre::eyre!("{} has an empty expected answer type.", path.display()));
	}

	for claim in &job.expected_answer.must_include {
		if claim.text().trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty expected claim.", path.display()));
		}
	}
	for claim in &job.expected_answer.must_not_include {
		if claim.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty forbidden claim.", path.display()));
		}
	}
	for phrase in &job.expected_answer.accepted_alternates {
		if phrase.is_null() {
			return Err(eyre::eyre!("{} has a null accepted alternate.", path.display()));
		}
	}

	Ok(())
}

fn validate_required_evidence(job: &RealWorldJob, path: &Path) -> Result<()> {
	let evidence_ids = corpus_evidence_ids(job);

	for evidence in &job.required_evidence {
		if evidence.claim_id.trim().is_empty() || evidence.requirement.trim().is_empty() {
			return Err(eyre::eyre!("{} has incomplete required evidence.", path.display()));
		}

		ensure_known_evidence(path, &evidence_ids, evidence.evidence_id.as_str())?;

		if evidence.quote.is_none() && evidence.selector.is_none() {
			return Err(eyre::eyre!(
				"{} required evidence {} must provide quote or selector.",
				path.display(),
				evidence.evidence_id
			));
		}
	}
	for (claim_id, link) in &job.expected_answer.evidence_links {
		if claim_id.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty evidence link claim id.", path.display()));
		}

		for evidence_id in link.ids() {
			ensure_known_evidence(path, &evidence_ids, evidence_id.as_str())?;
		}
	}

	Ok(())
}

fn validate_scoring_rubric(job: &RealWorldJob, path: &Path) -> Result<()> {
	if !(0.0..=1.0).contains(&job.scoring_rubric.pass_threshold) {
		return Err(eyre::eyre!("{} has invalid pass_threshold.", path.display()));
	}
	if job.scoring_rubric.dimensions.is_empty() {
		return Err(eyre::eyre!("{} has no scoring dimensions.", path.display()));
	}

	for (dimension_id, dimension) in &job.scoring_rubric.dimensions {
		if dimension_id.trim().is_empty()
			|| !dimension.weight.is_finite()
			|| !dimension.max_points.is_finite()
			|| dimension.weight <= 0.0
			|| dimension.max_points <= 0.0
			|| dimension.criteria.is_null()
		{
			return Err(eyre::eyre!(
				"{} has invalid scoring dimension {}.",
				path.display(),
				dimension_id
			));
		}
	}
	for rule in &job.scoring_rubric.hard_fail_rules {
		if rule.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty hard fail rule.", path.display()));
		}
	}

	Ok(())
}

fn validate_allowed_uncertainty(job: &RealWorldJob, path: &Path) -> Result<()> {
	if job.allowed_uncertainty.fallback_action.trim().is_empty() {
		return Err(eyre::eyre!("{} has an empty fallback action.", path.display()));
	}
	if job.allowed_uncertainty.can_answer_unknown
		&& job.allowed_uncertainty.acceptable_phrases.is_empty()
	{
		return Err(eyre::eyre!(
			"{} allows unknown answers but defines no acceptable uncertainty phrase.",
			path.display()
		));
	}

	for phrase in &job.allowed_uncertainty.acceptable_phrases {
		if phrase.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty uncertainty phrase.", path.display()));
		}
	}

	Ok(())
}

fn validate_required_rfc3339(value: &str, path: &Path, id: &str) -> Result<()> {
	if OffsetDateTime::parse(value, &Rfc3339).is_err() {
		return Err(eyre::eyre!("{} has invalid RFC3339 timestamp for {}.", path.display(), id));
	}

	Ok(())
}

fn validate_optional_rfc3339(value: &str, path: &Path, id: &str) -> Result<()> {
	if !value.trim().is_empty() {
		validate_required_rfc3339(value, path, id)?;
	}

	Ok(())
}

fn ensure_known_evidence(path: &Path, known: &BTreeSet<String>, evidence_id: &str) -> Result<()> {
	if !known.contains(evidence_id) {
		return Err(eyre::eyre!(
			"{} references unknown evidence id {}.",
			path.display(),
			evidence_id
		));
	}

	Ok(())
}

fn corpus_evidence_ids(job: &RealWorldJob) -> BTreeSet<String> {
	job.corpus.items.iter().map(|item| item.evidence_id.clone()).collect()
}

fn build_report(jobs: &[RealWorldJob], args: &RunArgs) -> Result<RealWorldReport> {
	if jobs.is_empty() {
		return Err(eyre::eyre!("At least one real_world_job fixture is required."));
	}

	let mut job_reports = Vec::with_capacity(jobs.len());
	let mut unsupported_claims = Vec::new();

	for job in jobs {
		let scoring = score_job(job);

		unsupported_claims.extend(scoring.unsupported_claims.clone());
		job_reports.push(job_report(job, scoring));
	}

	let suites = suite_reports(&job_reports);
	let not_encoded_suites = suites
		.iter()
		.filter(|suite| suite.status == TypedStatus::NotEncoded)
		.map(|suite| suite.suite_id.clone())
		.collect::<Vec<_>>();
	let summary = report_summary(&job_reports, &suites);

	Ok(RealWorldReport {
		schema: REPORT_SCHEMA.to_string(),
		run_id: args.run_id.clone(),
		generated_at: OffsetDateTime::now_utc().format(&Rfc3339)?,
		runner_version: VERSION.to_string(),
		corpus_profile: corpus_profile(jobs),
		adapter: adapter_report(args),
		summary,
		suites,
		jobs: job_reports,
		unsupported_claims,
		not_encoded_suites,
		private_corpus_redaction: private_corpus_redaction(jobs),
	})
}

fn score_job(job: &RealWorldJob) -> JobScoring {
	let answer = produced_answer(job);
	let produced_evidence = produced_evidence_ids(answer);
	let missing_claims = missing_required_claims(job, answer);
	let forbidden_claims = forbidden_claim_hits(job, answer);
	let missing_evidence = missing_required_evidence(job, &produced_evidence);
	let trap_ids_used = trap_ids_used(job, &produced_evidence);
	let mut unsupported_claims = unsupported_claims(job, answer);
	let hard_fail_hits = hard_fail_hits(job, &unsupported_claims, &trap_ids_used);
	let counts = FailureCounts {
		missing_claims: missing_claims.len(),
		forbidden_claims: forbidden_claims.len(),
		missing_evidence: missing_evidence.len(),
		trap_uses: trap_ids_used.len(),
		unsupported_claims: unsupported_claims.len(),
	};
	let dimension_scores = dimension_scores(job, &counts);
	let normalized_score = normalized_score(&dimension_scores);
	let wrong_result_count = counts.missing_claims
		+ counts.forbidden_claims
		+ counts.missing_evidence
		+ counts.trap_uses;
	let status = job_status(
		normalized_score,
		job.scoring_rubric.pass_threshold,
		wrong_result_count,
		unsupported_claims.len(),
	);
	let reason = job_reason(status, &counts, normalized_score);

	for claim in &mut unsupported_claims {
		claim.suite_id = job.suite.clone();
		claim.job_id = job.job_id.clone();
	}

	JobScoring {
		status,
		normalized_score,
		hard_fail_hits,
		unsupported_claims,
		wrong_result_count,
		trap_ids_used,
		dimension_scores,
		reason,
	}
}

fn produced_answer(job: &RealWorldJob) -> &ProducedAnswer {
	job.corpus
		.adapter_response
		.as_ref()
		.map(|response| &response.answer)
		.unwrap_or_else(|| synthetic_answer(job))
}

fn synthetic_answer(job: &RealWorldJob) -> &ProducedAnswer {
	let _ = job;

	static EMPTY_ANSWER: std::sync::OnceLock<ProducedAnswer> = std::sync::OnceLock::new();

	EMPTY_ANSWER.get_or_init(|| ProducedAnswer {
		content: String::new(),
		claims: Vec::new(),
		evidence_ids: Vec::new(),
		latency_ms: None,
		cost: None,
	})
}

fn produced_evidence_ids(answer: &ProducedAnswer) -> BTreeSet<String> {
	let mut evidence = answer.evidence_ids.iter().cloned().collect::<BTreeSet<_>>();

	for claim in &answer.claims {
		evidence.extend(claim.evidence_ids.iter().cloned());
	}

	evidence
}

fn missing_required_claims(job: &RealWorldJob, answer: &ProducedAnswer) -> Vec<String> {
	job.expected_answer
		.must_include
		.iter()
		.filter(|claim| !claim_is_present(claim, answer))
		.map(|claim| claim.text().to_string())
		.collect()
}

fn claim_is_present(claim: &ExpectedClaim, answer: &ProducedAnswer) -> bool {
	if let Some(claim_id) = claim.claim_id()
		&& answer.claims.iter().any(|produced| produced.claim_id.as_deref() == Some(claim_id))
	{
		return true;
	}

	answer.content.contains(claim.text())
}

fn forbidden_claim_hits(job: &RealWorldJob, answer: &ProducedAnswer) -> Vec<String> {
	job.expected_answer
		.must_not_include
		.iter()
		.filter(|claim| answer.content.contains(claim.as_str()))
		.cloned()
		.collect()
}

fn missing_required_evidence(
	job: &RealWorldJob,
	produced_evidence: &BTreeSet<String>,
) -> Vec<String> {
	job.required_evidence
		.iter()
		.filter(|evidence| {
			is_required_use(evidence) && !produced_evidence.contains(&evidence.evidence_id)
		})
		.map(|evidence| evidence.evidence_id.clone())
		.collect()
}

fn is_required_use(evidence: &RequiredEvidence) -> bool {
	matches!(evidence.requirement.as_str(), "cite" | "use" | "explain")
}

fn trap_ids_used(job: &RealWorldJob, produced_evidence: &BTreeSet<String>) -> Vec<String> {
	job.negative_traps
		.iter()
		.filter(|trap| trap.failure_if_used)
		.filter(|trap| {
			trap.evidence_ids.iter().any(|evidence_id| produced_evidence.contains(evidence_id))
		})
		.map(|trap| trap.trap_id.clone())
		.collect()
}

fn unsupported_claims(job: &RealWorldJob, answer: &ProducedAnswer) -> Vec<UnsupportedClaimReport> {
	answer.claims.iter().filter_map(|claim| unsupported_claim(job, claim)).collect()
}

fn unsupported_claim(job: &RealWorldJob, claim: &ProducedClaim) -> Option<UnsupportedClaimReport> {
	let Some(claim_id) = claim.claim_id.as_deref() else {
		return Some(unsupported_claim_report(claim, "claim has no claim_id"));
	};
	let Some(allowed) = job.expected_answer.evidence_links.get(claim_id).map(EvidenceLink::ids)
	else {
		return Some(unsupported_claim_report(
			claim,
			"claim_id is not present in expected_answer.evidence_links",
		));
	};

	if claim.evidence_ids.is_empty() {
		return Some(unsupported_claim_report(claim, "claim has no produced evidence ids"));
	}
	if !claim.evidence_ids.iter().any(|evidence_id| allowed.contains(evidence_id)) {
		return Some(unsupported_claim_report(
			claim,
			"claim evidence is not allowed for this claim_id",
		));
	}

	None
}

fn unsupported_claim_report(claim: &ProducedClaim, reason: &str) -> UnsupportedClaimReport {
	UnsupportedClaimReport {
		suite_id: String::new(),
		job_id: String::new(),
		claim_id: claim.claim_id.clone(),
		claim_text: bounded_text(claim.text.as_str(), 240),
		reason: reason.to_string(),
		evidence_ids: claim.evidence_ids.clone(),
	}
}

fn hard_fail_hits(
	job: &RealWorldJob,
	unsupported_claims: &[UnsupportedClaimReport],
	trap_ids_used: &[String],
) -> Vec<String> {
	let mut hits = Vec::new();

	if !unsupported_claims.is_empty() {
		hits.push(
			"unsupported high-confidence claim about a required decision or fact".to_string(),
		);
	}
	if !trap_ids_used.is_empty() {
		hits.push("use of a negative trap marked failure_if_used = true".to_string());
	}
	if job.expected_answer.requires_caveat && !answer_has_required_caveat(job, produced_answer(job))
	{
		hits.push("missing required caveat".to_string());
	}
	if job.expected_answer.requires_refusal && !answer_looks_like_refusal(produced_answer(job)) {
		hits.push("missing required refusal".to_string());
	}

	hits
}

fn answer_has_required_caveat(job: &RealWorldJob, answer: &ProducedAnswer) -> bool {
	job.allowed_uncertainty.acceptable_phrases.iter().any(|phrase| answer.content.contains(phrase))
}

fn answer_looks_like_refusal(answer: &ProducedAnswer) -> bool {
	let lower = answer.content.to_ascii_lowercase();

	lower.contains("cannot") || lower.contains("can't") || lower.contains("refuse")
}

fn dimension_scores(job: &RealWorldJob, counts: &FailureCounts) -> Vec<DimensionScoreReport> {
	job.scoring_rubric
		.dimensions
		.iter()
		.map(|(dimension_id, dimension)| DimensionScoreReport {
			dimension: dimension_id.clone(),
			score: dimension_score(dimension_id, dimension.max_points, counts),
			max_points: dimension.max_points,
			weight: dimension.weight,
		})
		.collect()
}

fn dimension_score(dimension_id: &str, max_points: f64, counts: &FailureCounts) -> f64 {
	let failed = match dimension_id {
		"answer_correctness" | "workflow_helpfulness" =>
			counts.missing_claims > 0 || counts.forbidden_claims > 0,
		"evidence_grounding" => counts.missing_evidence > 0 || counts.unsupported_claims > 0,
		"trap_avoidance" => counts.trap_uses > 0,
		"uncertainty_handling" => counts.unsupported_claims > 0,
		"lifecycle_behavior" => false,
		"debuggability" | "latency_resource" | "personalization_fit" =>
			counts.missing_claims > 0 || counts.unsupported_claims > 0,
		_ => counts.missing_claims > 0 || counts.unsupported_claims > 0 || counts.trap_uses > 0,
	};

	if failed { 0.0 } else { max_points }
}

fn normalized_score(scores: &[DimensionScoreReport]) -> f64 {
	let total_weight = scores.iter().map(|score| score.weight).sum::<f64>();

	if total_weight == 0.0 {
		return 0.0;
	}

	scores.iter().map(|score| (score.score / score.max_points) * score.weight).sum::<f64>()
		/ total_weight
}

fn job_status(
	normalized_score: f64,
	pass_threshold: f64,
	wrong_result_count: usize,
	unsupported_claim_count: usize,
) -> TypedStatus {
	if unsupported_claim_count > 0 {
		TypedStatus::UnsupportedClaim
	} else if wrong_result_count > 0 {
		TypedStatus::WrongResult
	} else if normalized_score >= pass_threshold {
		TypedStatus::Pass
	} else {
		TypedStatus::WrongResult
	}
}

fn job_reason(status: TypedStatus, counts: &FailureCounts, normalized_score: f64) -> String {
	match status {
		TypedStatus::Pass => format!("Job passed with normalized_score {normalized_score:.3}."),
		TypedStatus::UnsupportedClaim => format!(
			"Job produced {} unsupported claim(s), {} wrong-result signal(s), and normalized_score {normalized_score:.3}.",
			counts.unsupported_claims,
			counts.missing_claims
				+ counts.forbidden_claims
				+ counts.missing_evidence
				+ counts.trap_uses
		),
		TypedStatus::WrongResult => format!(
			"Job produced {} wrong-result signal(s) and normalized_score {normalized_score:.3}.",
			counts.missing_claims
				+ counts.forbidden_claims
				+ counts.missing_evidence
				+ counts.trap_uses
		),
		_ => "Job did not reach a runnable scoring state.".to_string(),
	}
}

fn job_report(job: &RealWorldJob, scoring: JobScoring) -> JobReport {
	let answer = produced_answer(job);
	let metrics = job_metrics(job, answer);

	JobReport {
		suite_id: job.suite.clone(),
		job_id: job.job_id.clone(),
		title: job.title.clone(),
		status: scoring.status,
		normalized_score: round3(scoring.normalized_score),
		hard_fail_hits: scoring.hard_fail_hits,
		expected_evidence: expected_evidence_report(job),
		produced_answer: answer.content.clone(),
		produced_evidence: produced_evidence_ids(answer).into_iter().collect(),
		unsupported_claim_count: scoring.unsupported_claims.len(),
		wrong_result_count: scoring.wrong_result_count,
		latency_ms: answer.latency_ms,
		cost: answer.cost.clone(),
		trap_ids_used: scoring.trap_ids_used,
		dimension_scores: scoring.dimension_scores,
		reason: scoring.reason,
		evidence_required_count: metrics.evidence_required_count,
		evidence_covered_count: metrics.evidence_covered_count,
		source_ref_required_count: metrics.source_ref_required_count,
		source_ref_covered_count: metrics.source_ref_covered_count,
		quote_required_count: metrics.quote_required_count,
		quote_covered_count: metrics.quote_covered_count,
		stale_retrieval_count: metrics.stale_retrieval_count,
		scope_check_count: metrics.scope_check_count,
		scope_correct_count: metrics.scope_correct_count,
		scope_violation_count: metrics.scope_violation_count,
		redaction_leak_count: metrics.redaction_leak_count,
		qdrant_rebuild_case: metrics.qdrant_rebuild_case,
	}
}

fn job_metrics(job: &RealWorldJob, answer: &ProducedAnswer) -> JobMetrics {
	let produced_evidence = produced_evidence_ids(answer);
	let source_ref_by_evidence = source_ref_by_evidence(job);
	let evidence_required_count =
		job.required_evidence.iter().filter(|evidence| is_required_use(evidence)).count();
	let evidence_covered_count = job
		.required_evidence
		.iter()
		.filter(|evidence| is_required_use(evidence))
		.filter(|evidence| produced_evidence.contains(&evidence.evidence_id))
		.count();
	let source_ref_required_count = evidence_required_count;
	let source_ref_covered_count = job
		.required_evidence
		.iter()
		.filter(|evidence| is_required_use(evidence))
		.filter(|evidence| produced_evidence.contains(&evidence.evidence_id))
		.filter(|evidence| {
			source_ref_by_evidence.get(evidence.evidence_id.as_str()).is_some_and(|source_ref| {
				source_ref.as_object().is_some_and(|object| !object.is_empty())
			})
		})
		.count();
	let quote_required_count = job
		.required_evidence
		.iter()
		.filter(|evidence| is_required_use(evidence) && evidence.quote.is_some())
		.count();
	let quote_covered_count = job
		.required_evidence
		.iter()
		.filter(|evidence| is_required_use(evidence) && evidence.quote.is_some())
		.filter(|evidence| produced_evidence.contains(&evidence.evidence_id))
		.count();
	let stale_retrieval_count = trap_use_count(job, &produced_evidence, "stale_fact", answer);
	let scope_violation_count = trap_use_count(job, &produced_evidence, "near_duplicate", answer);
	let scope_check_count =
		job.negative_traps.iter().filter(|trap| trap.trap_type == "near_duplicate").count();
	let redaction_leak_count = trap_use_count(job, &produced_evidence, "privacy_leak", answer);
	let scope_correct_count = scope_check_count.saturating_sub(scope_violation_count);
	let qdrant_rebuild_case = job.tags.iter().any(|tag| tag == "qdrant_rebuild");

	JobMetrics {
		evidence_required_count,
		evidence_covered_count,
		source_ref_required_count,
		source_ref_covered_count,
		quote_required_count,
		quote_covered_count,
		stale_retrieval_count,
		scope_check_count,
		scope_correct_count,
		scope_violation_count,
		redaction_leak_count,
		qdrant_rebuild_case,
	}
}

fn source_ref_by_evidence(job: &RealWorldJob) -> BTreeMap<&str, &Value> {
	job.corpus.items.iter().map(|item| (item.evidence_id.as_str(), &item.source_ref)).collect()
}

fn trap_use_count(
	job: &RealWorldJob,
	produced_evidence: &BTreeSet<String>,
	trap_type: &str,
	answer: &ProducedAnswer,
) -> usize {
	job.negative_traps
		.iter()
		.filter(|trap| trap.failure_if_used && trap.trap_type == trap_type)
		.filter(|trap| trap_was_used(job, trap, produced_evidence, answer))
		.count()
}

fn trap_was_used(
	job: &RealWorldJob,
	trap: &NegativeTrap,
	produced_evidence: &BTreeSet<String>,
	answer: &ProducedAnswer,
) -> bool {
	trap.evidence_ids.iter().any(|evidence_id| {
		produced_evidence.contains(evidence_id)
			|| answer_contains_corpus_item(job, evidence_id, answer)
	})
}

fn answer_contains_corpus_item(
	job: &RealWorldJob,
	evidence_id: &str,
	answer: &ProducedAnswer,
) -> bool {
	job.corpus
		.items
		.iter()
		.find(|item| item.evidence_id == evidence_id)
		.and_then(|item| item.text.as_deref())
		.is_some_and(|text| !text.trim().is_empty() && answer.content.contains(text))
}

fn expected_evidence_report(job: &RealWorldJob) -> Vec<ExpectedEvidenceReport> {
	job.required_evidence
		.iter()
		.map(|evidence| ExpectedEvidenceReport {
			evidence_id: evidence.evidence_id.clone(),
			claim_id: evidence.claim_id.clone(),
			requirement: evidence.requirement.clone(),
		})
		.collect()
}

fn suite_reports(jobs: &[JobReport]) -> Vec<SuiteReport> {
	SUITES.iter().map(|suite_id| suite_report(suite_id, jobs)).collect()
}

fn suite_report(suite_id: &str, jobs: &[JobReport]) -> SuiteReport {
	let suite_jobs = jobs.iter().filter(|job| job.suite_id == suite_id).collect::<Vec<_>>();

	if suite_jobs.is_empty() {
		return SuiteReport {
			suite_id: suite_id.to_string(),
			status: TypedStatus::NotEncoded,
			encoded_job_count: 0,
			score_mean: None,
			unsupported_claim_count: 0,
			wrong_result_count: 0,
			reason: NOT_ENCODED_REASON.to_string(),
		};
	}

	let status = aggregate_status(&suite_jobs);
	let score_sum = suite_jobs.iter().map(|job| job.normalized_score).sum::<f64>();
	let unsupported_claim_count = suite_jobs.iter().map(|job| job.unsupported_claim_count).sum();
	let wrong_result_count = suite_jobs.iter().map(|job| job.wrong_result_count).sum();

	SuiteReport {
		suite_id: suite_id.to_string(),
		status,
		encoded_job_count: suite_jobs.len(),
		score_mean: Some(round3(score_sum / suite_jobs.len() as f64)),
		unsupported_claim_count,
		wrong_result_count,
		reason: suite_reason(status, suite_jobs.len()),
	}
}

fn aggregate_status(jobs: &[&JobReport]) -> TypedStatus {
	let statuses = jobs.iter().map(|job| job.status).collect::<BTreeSet<_>>();

	if statuses.contains(&TypedStatus::UnsupportedClaim) {
		TypedStatus::UnsupportedClaim
	} else if statuses.contains(&TypedStatus::LifecycleFail) {
		TypedStatus::LifecycleFail
	} else if statuses.contains(&TypedStatus::WrongResult) {
		TypedStatus::WrongResult
	} else if statuses.contains(&TypedStatus::Incomplete) {
		TypedStatus::Incomplete
	} else if statuses.contains(&TypedStatus::Blocked) {
		TypedStatus::Blocked
	} else if statuses.contains(&TypedStatus::Pass) {
		TypedStatus::Pass
	} else {
		TypedStatus::NotEncoded
	}
}

fn suite_reason(status: TypedStatus, encoded_job_count: usize) -> String {
	match status {
		TypedStatus::Pass => format!("All {encoded_job_count} encoded job(s) passed."),
		TypedStatus::UnsupportedClaim =>
			"At least one encoded job produced an unsupported claim.".to_string(),
		TypedStatus::WrongResult => "At least one encoded job returned a wrong result.".to_string(),
		TypedStatus::LifecycleFail =>
			"At least one encoded lifecycle-scored job failed lifecycle behavior.".to_string(),
		TypedStatus::Incomplete => "At least one encoded job could not complete.".to_string(),
		TypedStatus::Blocked => "At least one encoded job is blocked.".to_string(),
		TypedStatus::NotEncoded => NOT_ENCODED_REASON.to_string(),
	}
}

fn report_summary(jobs: &[JobReport], suites: &[SuiteReport]) -> ReportSummary {
	let evidence_required_count = jobs.iter().map(|job| job.evidence_required_count).sum();
	let evidence_covered_count = jobs.iter().map(|job| job.evidence_covered_count).sum();
	let source_ref_required_count = jobs.iter().map(|job| job.source_ref_required_count).sum();
	let source_ref_covered_count = jobs.iter().map(|job| job.source_ref_covered_count).sum();
	let quote_required_count = jobs.iter().map(|job| job.quote_required_count).sum();
	let quote_covered_count = jobs.iter().map(|job| job.quote_covered_count).sum();
	let scope_check_count = jobs.iter().map(|job| job.scope_check_count).sum();
	let scope_correct_count = jobs.iter().map(|job| job.scope_correct_count).sum();
	let mut summary = ReportSummary {
		job_count: jobs.len(),
		encoded_suite_count: suites
			.iter()
			.filter(|suite| suite.status != TypedStatus::NotEncoded)
			.count(),
		not_encoded: suites.iter().filter(|suite| suite.status == TypedStatus::NotEncoded).count(),
		unsupported_claim_count: jobs.iter().map(|job| job.unsupported_claim_count).sum(),
		wrong_result_count: jobs.iter().map(|job| job.wrong_result_count).sum(),
		mean_score: mean_score(jobs),
		mean_latency_ms: mean_latency(jobs),
		total_cost: total_cost(jobs),
		evidence_required_count,
		evidence_covered_count,
		evidence_coverage: ratio(evidence_covered_count, evidence_required_count),
		source_ref_required_count,
		source_ref_covered_count,
		source_ref_coverage: ratio(source_ref_covered_count, source_ref_required_count),
		quote_required_count,
		quote_covered_count,
		quote_coverage: ratio(quote_covered_count, quote_required_count),
		stale_retrieval_count: jobs.iter().map(|job| job.stale_retrieval_count).sum(),
		scope_check_count,
		scope_correct_count,
		scope_correctness: ratio(scope_correct_count, scope_check_count),
		scope_violation_count: jobs.iter().map(|job| job.scope_violation_count).sum(),
		redaction_leak_count: jobs.iter().map(|job| job.redaction_leak_count).sum(),
		qdrant_rebuild_case_count: jobs.iter().filter(|job| job.qdrant_rebuild_case).count(),
		qdrant_rebuild_pass_count: jobs
			.iter()
			.filter(|job| job.qdrant_rebuild_case && job.status == TypedStatus::Pass)
			.count(),
		..ReportSummary::default()
	};

	for job in jobs {
		match job.status {
			TypedStatus::Pass => summary.pass += 1,
			TypedStatus::WrongResult => summary.wrong_result += 1,
			TypedStatus::LifecycleFail => summary.lifecycle_fail += 1,
			TypedStatus::Incomplete => summary.incomplete += 1,
			TypedStatus::Blocked => summary.blocked += 1,
			TypedStatus::NotEncoded => summary.not_encoded += 1,
			TypedStatus::UnsupportedClaim => summary.unsupported_claim += 1,
		}
	}

	summary
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
	if denominator == 0 {
		return 0.0;
	}

	round3(numerator as f64 / denominator as f64)
}

fn mean_score(jobs: &[JobReport]) -> f64 {
	if jobs.is_empty() {
		return 0.0;
	}

	round3(jobs.iter().map(|job| job.normalized_score).sum::<f64>() / jobs.len() as f64)
}

fn mean_latency(jobs: &[JobReport]) -> Option<f64> {
	let latencies = jobs.iter().filter_map(|job| job.latency_ms).collect::<Vec<_>>();

	if latencies.is_empty() {
		return None;
	}

	Some(round3(latencies.iter().sum::<f64>() / latencies.len() as f64))
}

fn total_cost(jobs: &[JobReport]) -> Option<CostReport> {
	let costs = jobs.iter().filter_map(|job| job.cost.as_ref()).collect::<Vec<_>>();

	if costs.is_empty() {
		return None;
	}

	let currency = costs.iter().find_map(|cost| cost.currency.clone());
	let amount = sum_optional_f64(costs.iter().filter_map(|cost| cost.amount));
	let input_tokens = sum_optional_u64(costs.iter().filter_map(|cost| cost.input_tokens));
	let output_tokens = sum_optional_u64(costs.iter().filter_map(|cost| cost.output_tokens));

	Some(CostReport { currency, amount, input_tokens, output_tokens })
}

fn sum_optional_f64(values: impl Iterator<Item = f64>) -> Option<f64> {
	let values = values.collect::<Vec<_>>();

	if values.is_empty() { None } else { Some(round3(values.iter().sum())) }
}

fn sum_optional_u64(values: impl Iterator<Item = u64>) -> Option<u64> {
	let values = values.collect::<Vec<_>>();

	if values.is_empty() { None } else { Some(values.iter().sum()) }
}

fn corpus_profile(jobs: &[RealWorldJob]) -> String {
	let profiles = jobs.iter().map(|job| job.corpus.profile.as_str()).collect::<BTreeSet<_>>();

	if profiles.len() == 1 {
		profiles.into_iter().next().unwrap_or("unknown").to_string()
	} else {
		"mixed".to_string()
	}
}

fn adapter_report(args: &RunArgs) -> AdapterReport {
	AdapterReport {
		adapter_id: args.adapter_id.clone(),
		name: args.adapter_name.clone(),
		behavior: "offline_fixture_response".to_string(),
		storage: TypedStatus::NotEncoded,
		runtime: TypedStatus::NotEncoded,
		notes: "Smoke runner scores checked-in fixture responses; it does not exercise a live external adapter.".to_string(),
	}
}

fn private_corpus_redaction(jobs: &[RealWorldJob]) -> PrivateCorpusRedaction {
	let private_fixture_count = jobs
		.iter()
		.filter(|job| matches!(job.corpus.profile, CorpusProfile::PrivateSanitized))
		.count();
	let policy = if private_fixture_count == 0 {
		"no_private_corpus".to_string()
	} else {
		"publish evidence ids and bounded score summaries only; do not publish private text"
			.to_string()
	};

	PrivateCorpusRedaction { policy, private_fixture_count }
}

fn render_markdown(report: &RealWorldReport, report_path: &Path) -> String {
	let report_path = report_path.display().to_string();
	let mut out = String::new();

	render_markdown_header(&mut out, report, report_path.as_str());
	render_markdown_suites(&mut out, report);
	render_markdown_jobs(&mut out, report);
	render_markdown_unsupported_claims(&mut out, report);
	render_markdown_semantics(&mut out, report);

	out
}

fn render_markdown_header(out: &mut String, report: &RealWorldReport, report_path: &str) {
	out.push_str("# Real-World Job Benchmark Report\n\n");
	out.push_str(
		"Goal: Publish a Markdown summary for one generated real_world_job benchmark report.\n",
	);
	out.push_str(
		"Read this when: You need a durable smoke report for real-world agent memory job fixtures.\n",
	);
	out.push_str(&format!("Inputs: `{}`.\n", md_inline(report_path)));
	out.push_str("Depends on: `apps/elf-eval/fixtures/real_world_job/`, `docs/spec/real_world_agent_memory_benchmark_v1.md`, and `Makefile.toml`.\n");
	out.push_str(
		"Verification: Compare this Markdown summary with the source JSON before committing.\n\n",
	);
	out.push_str("## Summary\n\n");
	out.push_str(&format!("- Run ID: `{}`\n", md_inline(report.run_id.as_str())));
	out.push_str(&format!("- Generated at: `{}`\n", md_inline(report.generated_at.as_str())));
	out.push_str(&format!("- Runner version: `{}`\n", md_inline(report.runner_version.as_str())));
	out.push_str(&format!("- Corpus profile: `{}`\n", md_inline(report.corpus_profile.as_str())));
	out.push_str(&format!(
		"- Adapter: `{}` ({})\n",
		md_inline(report.adapter.adapter_id.as_str()),
		md_inline(report.adapter.behavior.as_str())
	));
	out.push_str(&format!("- Jobs: `{}`\n", report.summary.job_count));
	out.push_str(&format!("- Encoded suites: `{}`\n", report.summary.encoded_suite_count));
	out.push_str(&format!("- Not-encoded suites: `{}`\n", report.not_encoded_suites.len()));
	out.push_str(&format!("- Status summary: `{}` pass, `{}` wrong_result, `{}` lifecycle_fail, `{}` incomplete, `{}` blocked, `{}` unsupported_claim\n", report.summary.pass, report.summary.wrong_result, report.summary.lifecycle_fail, report.summary.incomplete, report.summary.blocked, report.summary.unsupported_claim));
	out.push_str(&format!(
		"- Unsupported claim count: `{}`\n",
		report.summary.unsupported_claim_count
	));
	out.push_str(&format!("- Wrong-result count: `{}`\n", report.summary.wrong_result_count));
	out.push_str(&format!(
		"- Evidence coverage: `{}/{}` (`{:.3}`)\n",
		report.summary.evidence_covered_count,
		report.summary.evidence_required_count,
		report.summary.evidence_coverage
	));
	out.push_str(&format!(
		"- Source-ref coverage: `{}/{}` (`{:.3}`)\n",
		report.summary.source_ref_covered_count,
		report.summary.source_ref_required_count,
		report.summary.source_ref_coverage
	));
	out.push_str(&format!(
		"- Quote coverage: `{}/{}` (`{:.3}`)\n",
		report.summary.quote_covered_count,
		report.summary.quote_required_count,
		report.summary.quote_coverage
	));
	out.push_str(&format!("- Stale retrieval count: `{}`\n", report.summary.stale_retrieval_count));
	out.push_str(&format!(
		"- Scope correctness: `{}/{}` (`{:.3}`), violations `{}`\n",
		report.summary.scope_correct_count,
		report.summary.scope_check_count,
		report.summary.scope_correctness,
		report.summary.scope_violation_count
	));
	out.push_str(&format!("- Redaction leak count: `{}`\n", report.summary.redaction_leak_count));
	out.push_str(&format!(
		"- Qdrant rebuild cases: `{}` encoded, `{}` pass\n",
		report.summary.qdrant_rebuild_case_count, report.summary.qdrant_rebuild_pass_count
	));
	out.push_str(&format!("- Mean score: `{:.3}`\n", report.summary.mean_score));
	out.push_str(&format!(
		"- Mean latency: `{}`\n",
		optional_f64(report.summary.mean_latency_ms, " ms")
	));
	out.push_str(&format!("- Cost: `{}`\n", cost_display(report.summary.total_cost.as_ref())));
	out.push_str(&format!(
		"- Private corpus redaction: `{}`\n\n",
		md_inline(report.private_corpus_redaction.policy.as_str())
	));
}

fn render_markdown_suites(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Suites\n\n");
	out.push_str(
		"| Suite | Status | Jobs | Score | Unsupported Claims | Wrong Results | Reason |\n",
	);
	out.push_str("| --- | --- | ---: | ---: | ---: | ---: | --- |\n");

	for suite in &report.suites {
		out.push_str(&format!(
			"| {} | `{}` | {} | `{}` | {} | {} | {} |\n",
			md_cell(suite.suite_id.as_str()),
			status_str(suite.status),
			suite.encoded_job_count,
			optional_f64(suite.score_mean, ""),
			suite.unsupported_claim_count,
			suite.wrong_result_count,
			md_cell(suite.reason.as_str())
		));
	}

	out.push('\n');
}

fn render_markdown_jobs(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Jobs\n\n");
	out.push_str("| Suite | Job | Status | Score | Expected Evidence | Produced Evidence | Unsupported Claims | Wrong Results | Latency | Cost |\n");
	out.push_str("| --- | --- | --- | ---: | --- | --- | ---: | ---: | ---: | --- |\n");

	for job in &report.jobs {
		let expected = job
			.expected_evidence
			.iter()
			.map(|evidence| evidence.evidence_id.as_str())
			.collect::<Vec<_>>()
			.join(", ");
		let produced = job.produced_evidence.join(", ");

		out.push_str(&format!(
			"| {} | {} | `{}` | `{:.3}` | `{}` | `{}` | {} | {} | `{}` | `{}` |\n",
			md_cell(job.suite_id.as_str()),
			md_cell(job.job_id.as_str()),
			status_str(job.status),
			job.normalized_score,
			md_inline(expected.as_str()),
			md_inline(produced.as_str()),
			job.unsupported_claim_count,
			job.wrong_result_count,
			optional_f64(job.latency_ms, " ms"),
			cost_display(job.cost.as_ref())
		));
	}

	out.push('\n');
}

fn render_markdown_unsupported_claims(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Unsupported Claims\n\n");

	if report.unsupported_claims.is_empty() {
		out.push_str("No unsupported claims were produced by encoded jobs.\n\n");

		return;
	}

	out.push_str("| Suite | Job | Claim | Evidence | Reason |\n");
	out.push_str("| --- | --- | --- | --- | --- |\n");

	for claim in &report.unsupported_claims {
		out.push_str(&format!(
			"| {} | {} | {} | `{}` | {} |\n",
			md_cell(claim.suite_id.as_str()),
			md_cell(claim.job_id.as_str()),
			md_cell(claim.claim_text.as_str()),
			md_inline(claim.evidence_ids.join(", ").as_str()),
			md_cell(claim.reason.as_str())
		));
	}

	out.push('\n');
}

fn render_markdown_semantics(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Result Semantics\n\n");
	out.push_str(
		"This report uses `docs/spec/real_world_agent_memory_benchmark_v1.md` status terms.\n",
	);
	out.push_str("It is a real-world job fixture report, not a Docker live-baseline report.\n");
	out.push_str("Existing live-baseline reports remain valid for their encoded retrieval and lifecycle checks and are not reinterpreted as real-world suite wins.\n\n");
	out.push_str(
		"The summary counters report required evidence coverage, source-ref coverage, quote coverage, stale retrievals, scope violations, redaction leaks, and Qdrant rebuild case coverage across encoded jobs.\n\n",
	);
	out.push_str(
		"- `pass`: encoded jobs met their pass threshold with required evidence and no hard-fail rule.\n",
	);
	out.push_str(
		"- `wrong_result`: a job completed but missed required answer or evidence expectations.\n",
	);
	out.push_str("- `unsupported_claim`: a job produced a substantive claim not supported by the fixture evidence links.\n");
	out.push_str("- `not_encoded`: a suite has no checked-in real_world_job fixture, so no pass/fail claim is allowed.\n\n");
	out.push_str("## Not-Encoded Suites\n\n");

	if report.not_encoded_suites.is_empty() {
		out.push_str("All declared suites have at least one encoded job.\n");
	} else {
		for suite in &report.not_encoded_suites {
			out.push_str(&format!("- `{}`\n", md_inline(suite.as_str())));
		}
	}
}

fn status_str(status: TypedStatus) -> &'static str {
	match status {
		TypedStatus::Pass => "pass",
		TypedStatus::WrongResult => "wrong_result",
		TypedStatus::LifecycleFail => "lifecycle_fail",
		TypedStatus::Incomplete => "incomplete",
		TypedStatus::Blocked => "blocked",
		TypedStatus::NotEncoded => "not_encoded",
		TypedStatus::UnsupportedClaim => "unsupported_claim",
	}
}

fn write_or_print(path: Option<&Path>, content: &str) -> Result<()> {
	if let Some(path) = path {
		if let Some(parent) = path.parent()
			&& !parent.as_os_str().is_empty()
		{
			fs::create_dir_all(parent)?;
		}

		fs::write(path, content)?;

		println!("Wrote {}", path.display());
	} else {
		println!("{content}");
	}

	Ok(())
}

fn optional_f64(value: Option<f64>, suffix: &str) -> String {
	value.map(|value| format!("{value:.3}{suffix}")).unwrap_or_else(|| "-".to_string())
}

fn cost_display(cost: Option<&CostReport>) -> String {
	let Some(cost) = cost else {
		return "-".to_string();
	};

	match (cost.amount, cost.currency.as_deref()) {
		(Some(amount), Some(currency)) => format!("{amount:.3} {currency}"),
		(Some(amount), None) => format!("{amount:.3}"),
		(None, _) => "-".to_string(),
	}
}

fn bounded_text(value: &str, max_chars: usize) -> String {
	let mut chars = value.chars();
	let text = chars.by_ref().take(max_chars).collect::<String>();

	if chars.next().is_some() { format!("{text}...") } else { text }
}

fn md_inline(value: &str) -> String {
	value.replace('`', "'").replace('\n', " ")
}

fn md_cell(value: &str) -> String {
	md_inline(value).replace('|', "\\|")
}

fn round3(value: f64) -> f64 {
	(value * 1_000.0).round() / 1_000.0
}
