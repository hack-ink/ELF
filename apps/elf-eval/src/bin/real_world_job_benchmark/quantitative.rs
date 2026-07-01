use std::env;

mod product_manifest;

pub(super) use product_manifest::quantitative_product_manifest_from_report;

use crate::{
	AdapterReport, BTreeMap, BTreeSet, ExportQuantitativeAuditManifestArgs, JobReport, Path,
	PathBuf, QuantitativeAuditArtifact, QuantitativeAuditManifest, QuantitativeBenchmarkControls,
	QuantitativeBenchmarkReport, QuantitativeBenchmarkRow, QuantitativeConfidenceInterval,
	QuantitativePerQueryRow, RealWorldJob, ReportSummary, Result, eyre, formatting, fs, scoring,
};

use product_manifest::quantitative_product_manifest;

const QUANTITATIVE_SCOREBOARD_SCHEMA: &str = "elf.agent_memory_quantitative_benchmark/v1";
const QUANTITATIVE_PRODUCT_MANIFEST_SCHEMA: &str =
	"elf.agent_memory_quantitative_product_manifest/v1";
const QUANTITATIVE_AUDIT_MANIFEST_SCHEMA: &str = "elf.agent_memory_quantitative_audit_manifest/v1";
const REQUIRED_HELD_OUT_AUDIT_CONTROL: &str = "query_ids_locked_before_product_runtime";
const REQUIRED_QREL_LEAKAGE_AUDIT_CONTROL: &str =
	"product_runtime_did_not_receive_expected_answers_or_qrels";
const REQUIRED_CANDIDATE_LEAKAGE_AUDIT_CONTROL: &str =
	"ranked_candidates_emitted_by_product_runtime";
const QUANTITATIVE_K_VALUES: &[usize] = &[1, 3, 5, 10];
const MIN_LEADERBOARD_QUERY_COUNT: usize = 30;
const WILSON_95_Z: f64 = 1.959963984540054;
const QUANTITATIVE_ROW_CLAIM_BOUNDARY: &str = concat!(
	"Quantitative metrics are bounded to this generated report. ",
	"Fixture-backed rows prove benchmark mechanics, not product-runtime or leaderboard claims."
);

pub(super) struct QuantitativeReportInput<'a> {
	pub(super) run_id: &'a str,
	pub(super) generated_at: &'a str,
	pub(super) adapter: &'a AdapterReport,
	pub(super) source_jobs: &'a [RealWorldJob],
	pub(super) jobs: &'a [JobReport],
	pub(super) summary: &'a ReportSummary,
	pub(super) product_manifest_path: Option<&'a Path>,
	pub(super) audit_manifest_path: Option<&'a Path>,
}

struct QuantitativeAuditContext<'a> {
	run_id: &'a str,
	corpus_id: &'a str,
	product: &'a str,
	adapter_id: &'a str,
	source_jobs: &'a [RealWorldJob],
	ranking_query_count: usize,
	explicit_qrel_query_count: usize,
}

struct QuantitativeAuditEvidence {
	held_out: bool,
	leakage_audited: bool,
	audit_manifest_id: Option<String>,
}

pub(super) fn quantitative_scoreboard_report(
	input: QuantitativeReportInput<'_>,
) -> Result<QuantitativeBenchmarkReport> {
	let corpus_id = quantitative_corpus_id(input.source_jobs);
	let evidence_class = quantitative_evidence_class(input.adapter, input.jobs);
	let per_query_rows = quantitative_per_query_rows(
		input.source_jobs,
		input.jobs,
		corpus_id.as_str(),
		evidence_class,
		input.adapter.adapter_id.as_str(),
	);
	let ranking_query_count = per_query_rows
		.iter()
		.filter(|row| row.candidate_count > 0 && row.expected_relevant_count > 0)
		.count();
	let explicit_qrel_query_count =
		per_query_rows.iter().filter(|row| row.qrel_source == "explicit_qrels").count();
	let metric_comparable = ranking_query_count > 0;
	let result_state = quantitative_result_state(input.summary);
	let audit_evidence = quantitative_audit_evidence(
		input.audit_manifest_path,
		QuantitativeAuditContext {
			run_id: input.run_id,
			corpus_id: corpus_id.as_str(),
			product: "ELF",
			adapter_id: input.adapter.adapter_id.as_str(),
			source_jobs: input.source_jobs,
			ranking_query_count,
			explicit_qrel_query_count,
		},
	)?;
	let leaderboard_eligible = quantitative_row_leaderboard_eligible(
		evidence_class,
		input.source_jobs.len(),
		ranking_query_count,
		explicit_qrel_query_count,
		metric_comparable,
		&audit_evidence,
	);
	let row = QuantitativeBenchmarkRow {
		product: "ELF".to_string(),
		adapter_id: input.adapter.adapter_id.clone(),
		adapter_name: input.adapter.name.clone(),
		suite: quantitative_suite_id(input.jobs),
		evidence_class: evidence_class.to_string(),
		source_manifest_corpus_id: Some(corpus_id.clone()),
		result_state: result_state.to_string(),
		comparable: metric_comparable,
		metric_comparable,
		leaderboard_eligible,
		held_out: audit_evidence.held_out,
		leakage_audited: audit_evidence.leakage_audited,
		audit_manifest_id: audit_evidence.audit_manifest_id,
		fixture_regression_only: evidence_class == "fixture_backed",
		sample_size: input.jobs.len(),
		ranking_query_count,
		ranking_coverage_state: ranking_coverage_state(
			input.summary,
			input.source_jobs.len(),
			ranking_query_count,
		)
		.to_string(),
		ranked_candidate_source: ranked_candidate_source(ranking_query_count).to_string(),
		qrel_source: aggregate_qrel_source(ranking_query_count, explicit_qrel_query_count)
			.to_string(),
		explicit_qrel_query_count,
		metrics: aggregate_metrics(per_query_rows.as_slice()),
		metric_states: aggregate_metric_states(result_state, metric_comparable),
		denominators: aggregate_denominators(per_query_rows.as_slice()),
		confidence_intervals: aggregate_confidence_intervals(per_query_rows.as_slice()),
		claim_boundary: QUANTITATIVE_ROW_CLAIM_BOUNDARY.to_string(),
	};
	let product_manifest =
		quantitative_product_manifest(input.product_manifest_path, corpus_id.as_str())?;
	let imported_row_count = product_manifest.rows.len();
	let imported_per_query_count = product_manifest.per_query_rows.len();
	let mut rows = vec![row];
	let mut merged_per_query_rows = per_query_rows;

	rows.extend(product_manifest.rows);
	merged_per_query_rows.extend(product_manifest.per_query_rows);

	let leaderboard_claim_allowed = rows.iter().filter(|row| row.leaderboard_eligible).count() >= 2;
	let controls = QuantitativeBenchmarkControls {
		same_corpus_required: true,
		same_task_required: true,
		ranked_candidates_required_for_ranking_metrics: true,
		explicit_relevance_judgments_required_for_leaderboard: true,
		minimum_query_count_for_leaderboard: MIN_LEADERBOARD_QUERY_COUNT,
		current_query_count: input.source_jobs.len(),
		current_ranking_query_count: ranking_query_count,
		current_explicit_qrel_query_count: explicit_qrel_query_count,
		leaderboard_claim_allowed,
		leakage_control:
			"held_out_or_leakage_audited_runtime_rows_required_before_leaderboard_claims"
				.to_string(),
	};

	Ok(QuantitativeBenchmarkReport {
		schema: QUANTITATIVE_SCOREBOARD_SCHEMA.to_string(),
		generated_at: input.generated_at.to_string(),
		corpus_id,
		k_values: QUANTITATIVE_K_VALUES.to_vec(),
		rows,
		per_query_rows: merged_per_query_rows,
		metrics_not_encoded: quantitative_metrics_not_encoded(
			imported_row_count,
			imported_per_query_count,
		),
		controls,
		claim_boundary: concat!(
			"Do not convert fixture mechanics, missing explicit qrels, ",
			"or partial candidate coverage into product leaderboard claims."
		)
		.to_string(),
	})
}

pub(super) fn quantitative_audit_manifest_from_jobs(
	jobs: &[RealWorldJob],
	args: &ExportQuantitativeAuditManifestArgs,
) -> Result<QuantitativeAuditManifest> {
	let product = args.product.trim();
	let adapter_id = args.adapter_id.trim();

	if product.is_empty() || adapter_id.is_empty() {
		return Err(eyre::eyre!("quantitative audit export requires product and adapter_id."));
	}

	let corpus_id = quantitative_corpus_id(jobs);
	let ranking_query_count = ranking_query_count(jobs);
	let explicit_qrel_query_count = explicit_qrel_query_count(jobs);
	let manifest = QuantitativeAuditManifest {
		schema: QUANTITATIVE_AUDIT_MANIFEST_SCHEMA.to_string(),
		manifest_id: args
			.manifest_id
			.clone()
			.unwrap_or_else(|| format!("{}-quantitative-audit-manifest", args.run_id)),
		run_id: args.run_id.clone(),
		corpus_id,
		product: product.to_string(),
		adapter_id: adapter_id.to_string(),
		held_out: args.held_out,
		leakage_audited: args.leakage_audited,
		sample_size: jobs.len(),
		ranking_query_count,
		explicit_qrel_query_count,
		query_ids: ranking_query_ids(jobs).into_iter().map(str::to_string).collect(),
		controls: args.controls.clone(),
		artifacts: vec![QuantitativeAuditArtifact {
			role: "product_runtime_fixtures".to_string(),
			path: audit_artifact_display_path(args.fixtures.as_path()),
			sha256: fixture_path_digest(args.fixtures.as_path())?,
		}],
		claim_boundary: args.claim_boundary.clone().unwrap_or_else(|| {
			if args.held_out || args.leakage_audited {
				concat!(
					"Audit manifest supplied by operator; runner validates run/corpus/product/",
					"adapter/count/query-id/artifact bindings before opening row gates."
				)
				.to_string()
			} else {
				concat!(
					"Diagnostic audit manifest binds the current product-runtime fixture set to ",
					"query ids and counts, but it does not prove held-out or leakage-audited status."
				)
				.to_string()
			}
		}),
	};

	validate_quantitative_audit_manifest(
		&manifest,
		args.fixtures.as_path(),
		QuantitativeAuditContext {
			run_id: args.run_id.as_str(),
			corpus_id: manifest.corpus_id.as_str(),
			product,
			adapter_id,
			source_jobs: jobs,
			ranking_query_count: manifest.ranking_query_count,
			explicit_qrel_query_count: manifest.explicit_qrel_query_count,
		},
	)?;

	Ok(manifest)
}

fn quantitative_audit_evidence(
	path: Option<&Path>,
	context: QuantitativeAuditContext<'_>,
) -> Result<QuantitativeAuditEvidence> {
	let Some(path) = path else {
		return Ok(QuantitativeAuditEvidence {
			held_out: false,
			leakage_audited: false,
			audit_manifest_id: None,
		});
	};
	let raw = fs::read_to_string(path)?;
	let manifest = serde_json::from_str::<QuantitativeAuditManifest>(&raw).map_err(|err| {
		eyre::eyre!("Failed to parse quantitative audit manifest {}: {err}", path.display())
	})?;

	validate_quantitative_audit_manifest(&manifest, path, context)?;

	Ok(QuantitativeAuditEvidence {
		held_out: manifest.held_out,
		leakage_audited: manifest.leakage_audited,
		audit_manifest_id: Some(manifest.manifest_id),
	})
}

fn validate_quantitative_audit_manifest(
	manifest: &QuantitativeAuditManifest,
	path: &Path,
	context: QuantitativeAuditContext<'_>,
) -> Result<()> {
	if manifest.schema != QUANTITATIVE_AUDIT_MANIFEST_SCHEMA {
		return Err(eyre::eyre!(
			"{} has schema {}, expected {QUANTITATIVE_AUDIT_MANIFEST_SCHEMA}.",
			path.display(),
			manifest.schema
		));
	}
	if manifest.manifest_id.trim().is_empty() {
		return Err(eyre::eyre!("{} has an empty manifest_id.", path.display()));
	}
	if manifest.run_id != context.run_id {
		return Err(eyre::eyre!(
			"{} has run_id {}, expected {}.",
			path.display(),
			manifest.run_id,
			context.run_id
		));
	}
	if manifest.corpus_id != context.corpus_id {
		return Err(eyre::eyre!(
			"{} has corpus_id {}, expected {}.",
			path.display(),
			manifest.corpus_id,
			context.corpus_id
		));
	}
	if manifest.product != context.product || manifest.adapter_id != context.adapter_id {
		return Err(eyre::eyre!(
			"{} has product {}:{} but current row is {}:{}.",
			path.display(),
			manifest.product,
			manifest.adapter_id,
			context.product,
			context.adapter_id
		));
	}
	if manifest.sample_size != context.source_jobs.len() {
		return Err(eyre::eyre!(
			"{} has sample_size {}, expected {}.",
			path.display(),
			manifest.sample_size,
			context.source_jobs.len()
		));
	}
	if manifest.ranking_query_count != context.ranking_query_count {
		return Err(eyre::eyre!(
			"{} has ranking_query_count {}, expected {}.",
			path.display(),
			manifest.ranking_query_count,
			context.ranking_query_count
		));
	}
	if manifest.explicit_qrel_query_count != context.explicit_qrel_query_count {
		return Err(eyre::eyre!(
			"{} has explicit_qrel_query_count {}, expected {}.",
			path.display(),
			manifest.explicit_qrel_query_count,
			context.explicit_qrel_query_count
		));
	}

	validate_quantitative_audit_query_ids(manifest, path, context.source_jobs)?;
	validate_quantitative_audit_controls(manifest, path)?;

	validate_quantitative_audit_artifacts(manifest, path)
}

fn validate_quantitative_audit_query_ids(
	manifest: &QuantitativeAuditManifest,
	path: &Path,
	source_jobs: &[RealWorldJob],
) -> Result<()> {
	let expected = ranking_query_ids(source_jobs);
	let actual = manifest.query_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();

	if actual.len() != manifest.query_ids.len() {
		return Err(eyre::eyre!("{} has duplicate quantitative audit query_ids.", path.display()));
	}
	if actual != expected {
		let missing = expected.difference(&actual).copied().collect::<Vec<_>>();
		let extra = actual.difference(&expected).copied().collect::<Vec<_>>();

		return Err(eyre::eyre!(
			"{} audit query_ids do not match current ranked-query set; missing: {:?}, extra: {:?}.",
			path.display(),
			missing,
			extra
		));
	}

	Ok(())
}

fn validate_quantitative_audit_controls(
	manifest: &QuantitativeAuditManifest,
	path: &Path,
) -> Result<()> {
	let controls = manifest.controls.iter().map(String::as_str).collect::<BTreeSet<_>>();

	if manifest.held_out && !controls.contains(REQUIRED_HELD_OUT_AUDIT_CONTROL) {
		return Err(eyre::eyre!(
			"{} marks held_out=true without required control {}.",
			path.display(),
			REQUIRED_HELD_OUT_AUDIT_CONTROL
		));
	}
	if manifest.leakage_audited
		&& (!controls.contains(REQUIRED_QREL_LEAKAGE_AUDIT_CONTROL)
			|| !controls.contains(REQUIRED_CANDIDATE_LEAKAGE_AUDIT_CONTROL))
	{
		return Err(eyre::eyre!(
			"{} marks leakage_audited=true without required controls {} and {}.",
			path.display(),
			REQUIRED_QREL_LEAKAGE_AUDIT_CONTROL,
			REQUIRED_CANDIDATE_LEAKAGE_AUDIT_CONTROL
		));
	}
	if (manifest.held_out || manifest.leakage_audited) && manifest.claim_boundary.trim().is_empty()
	{
		return Err(eyre::eyre!(
			"{} marks audit controls true but has an empty claim_boundary.",
			path.display()
		));
	}

	Ok(())
}

fn validate_quantitative_audit_artifacts(
	manifest: &QuantitativeAuditManifest,
	path: &Path,
) -> Result<()> {
	if manifest.artifacts.is_empty() {
		return Err(eyre::eyre!("{} has no quantitative audit artifacts.", path.display()));
	}

	for artifact in &manifest.artifacts {
		if artifact.role.trim().is_empty()
			|| artifact.path.trim().is_empty()
			|| artifact.sha256.trim().is_empty()
		{
			return Err(eyre::eyre!(
				"{} has an incomplete quantitative audit artifact.",
				path.display()
			));
		}
		if artifact.sha256.len() != 64 || !artifact.sha256.chars().all(|ch| ch.is_ascii_hexdigit())
		{
			return Err(eyre::eyre!(
				"{} artifact {} has invalid sha256 digest {}.",
				path.display(),
				artifact.role,
				artifact.sha256
			));
		}

		let artifact_path = resolve_quantitative_audit_artifact_path(path, artifact.path.as_str());
		let actual = fixture_path_digest(artifact_path.as_path()).map_err(|err| {
			eyre::eyre!(
				"{} artifact {} could not be digested at {}: {err}",
				path.display(),
				artifact.role,
				artifact_path.display()
			)
		})?;

		if actual != artifact.sha256 {
			return Err(eyre::eyre!(
				"{} artifact {} sha256 mismatch for {}: manifest {}, actual {}.",
				path.display(),
				artifact.role,
				artifact_path.display(),
				artifact.sha256,
				actual
			));
		}
	}

	Ok(())
}

fn resolve_quantitative_audit_artifact_path(manifest_path: &Path, artifact_path: &str) -> PathBuf {
	let raw = PathBuf::from(artifact_path);

	if raw.is_absolute() {
		return raw;
	}

	let cwd_path = env::current_dir().map(|cwd| cwd.join(&raw)).unwrap_or_else(|_| raw.clone());

	if cwd_path.exists() {
		return cwd_path;
	}

	manifest_path.parent().map(|parent| parent.join(&raw)).unwrap_or(cwd_path)
}

fn quantitative_metrics_not_encoded(
	imported_row_count: usize,
	imported_per_query_count: usize,
) -> Vec<String> {
	let mut metrics =
		vec!["paired_significance".to_string(), "audit_manifest_validation".to_string()];

	if imported_row_count == 0 {
		metrics.push("external_product_manifest_import".to_string());
	}
	if imported_row_count > 0 && imported_per_query_count == 0 {
		metrics.push("imported_product_per_query_rows".to_string());
	}

	metrics
}

fn quantitative_per_query_rows(
	source_jobs: &[RealWorldJob],
	jobs: &[JobReport],
	corpus_id: &str,
	evidence_class: &str,
	adapter_id: &str,
) -> Vec<QuantitativePerQueryRow> {
	source_jobs
		.iter()
		.zip(jobs.iter())
		.map(|(source_job, job)| {
			quantitative_per_query_row(source_job, job, corpus_id, evidence_class, adapter_id)
		})
		.collect()
}

fn quantitative_per_query_row(
	source_job: &RealWorldJob,
	job: &JobReport,
	corpus_id: &str,
	evidence_class: &str,
	adapter_id: &str,
) -> QuantitativePerQueryRow {
	let relevance = relevance_grades(source_job, job);
	let candidates = scoring::produced_evidence_order(source_job);
	let positive_relevance_count = positive_qrel_count(&relevance);
	let metrics = per_query_metrics(candidates.as_slice(), &relevance);
	let metric_state = if positive_relevance_count == 0 || candidates.is_empty() {
		"not_encoded"
	} else {
		formatting::status_str(job.status)
	};
	let metric_states = metrics.keys().map(|key| (key.clone(), metric_state.to_string())).collect();
	let denominators = per_query_denominators(candidates.len(), positive_relevance_count);

	QuantitativePerQueryRow {
		job_id: job.job_id.clone(),
		suite: job.suite_id.clone(),
		evidence_class: evidence_class.to_string(),
		source_manifest_corpus_id: Some(corpus_id.to_string()),
		result_state: formatting::status_str(job.status).to_string(),
		expected_relevant_count: positive_relevance_count,
		candidate_count: candidates.len(),
		qrel_source: qrel_source(source_job, relevance.is_empty()).to_string(),
		relevance_grade_sum: formatting::round3(relevance.values().sum::<f64>()),
		product: "ELF".to_string(),
		adapter_id: adapter_id.to_string(),
		metrics,
		metric_states,
		denominators,
		claim_boundary: QUANTITATIVE_ROW_CLAIM_BOUNDARY.to_string(),
	}
}

fn relevance_grades(source_job: &RealWorldJob, job: &JobReport) -> BTreeMap<String, f64> {
	let explicit = source_job
		.expected_answer
		.relevance_judgments
		.iter()
		.map(|judgment| (judgment.evidence_id.clone(), judgment.grade))
		.collect::<BTreeMap<_, _>>();

	if !explicit.is_empty() {
		return explicit;
	}

	job.expected_evidence.iter().map(|evidence| (evidence.evidence_id.clone(), 1.0)).collect()
}

fn per_query_metrics(
	candidates: &[String],
	relevance: &BTreeMap<String, f64>,
) -> BTreeMap<String, Option<f64>> {
	let mut metrics = BTreeMap::new();

	for k in QUANTITATIVE_K_VALUES {
		let relevant_at_k = relevant_at_k(candidates, relevance, *k);

		metrics
			.insert(format!("recall_at_{k}"), rate(relevant_at_k, positive_qrel_count(relevance)));
		metrics.insert(format!("precision_at_{k}"), rate(relevant_at_k, *k));
		metrics.insert(
			format!("success_at_{k}"),
			Some(f64::from(relevant_at_k > 0 && positive_qrel_count(relevance) > 0)),
		);
	}

	metrics.insert("mrr".to_string(), reciprocal_rank(candidates, relevance));
	metrics.insert("ndcg_at_5".to_string(), ndcg_at_k(candidates, relevance, 5));
	metrics.insert("average_precision".to_string(), average_precision(candidates, relevance));

	metrics
}

fn relevant_at_k(candidates: &[String], relevance: &BTreeMap<String, f64>, k: usize) -> usize {
	candidates
		.iter()
		.take(k)
		.filter(|candidate| relevance.get(candidate.as_str()).is_some_and(|grade| *grade > 0.0))
		.count()
}

fn reciprocal_rank(candidates: &[String], relevance: &BTreeMap<String, f64>) -> Option<f64> {
	if positive_qrel_count(relevance) == 0 {
		return None;
	}

	Some(
		candidates
			.iter()
			.position(|candidate| {
				relevance.get(candidate.as_str()).is_some_and(|grade| *grade > 0.0)
			})
			.map_or(0.0, |index| 1.0 / (index + 1) as f64),
	)
}

fn ndcg_at_k(candidates: &[String], relevance: &BTreeMap<String, f64>, k: usize) -> Option<f64> {
	if positive_qrel_count(relevance) == 0 {
		return None;
	}

	let dcg = candidates
		.iter()
		.take(k)
		.enumerate()
		.map(|(index, candidate)| {
			relevance.get(candidate.as_str()).copied().unwrap_or(0.0).max(0.0)
				/ ((index + 2) as f64).log2()
		})
		.sum::<f64>();
	let mut ideal = relevance.values().copied().filter(|grade| *grade > 0.0).collect::<Vec<_>>();

	ideal.sort_by(|left, right| right.total_cmp(left));

	let idcg = ideal
		.iter()
		.take(k)
		.enumerate()
		.map(|(index, grade)| grade / ((index + 2) as f64).log2())
		.sum::<f64>();

	Some(if idcg > 0.0 { dcg / idcg } else { 0.0 })
}

fn average_precision(candidates: &[String], relevance: &BTreeMap<String, f64>) -> Option<f64> {
	let positive_count = positive_qrel_count(relevance);

	if positive_count == 0 {
		return None;
	}

	let mut hit_count = 0;
	let mut precision_sum = 0.0;
	let mut seen = BTreeSet::new();

	for (index, candidate) in candidates.iter().enumerate() {
		if !seen.insert(candidate.as_str()) {
			continue;
		}
		if relevance.get(candidate.as_str()).is_some_and(|grade| *grade > 0.0) {
			hit_count += 1;
			precision_sum += hit_count as f64 / (index + 1) as f64;
		}
	}

	Some(precision_sum / positive_count as f64)
}

fn aggregate_metrics(rows: &[QuantitativePerQueryRow]) -> BTreeMap<String, Option<f64>> {
	let mut sums = BTreeMap::<String, (f64, usize)>::new();
	let mut metrics = quantitative_metric_names()
		.into_iter()
		.map(|metric| (metric, None))
		.collect::<BTreeMap<_, _>>();

	for row in rows {
		for (metric, value) in &row.metrics {
			if let Some(value) = value {
				let (sum, count) = sums.entry(metric.clone()).or_default();

				*sum += *value;
				*count += 1;
			}
		}
	}
	for (metric, (sum, count)) in sums {
		metrics.insert(metric, (count > 0).then(|| formatting::round3(sum / count as f64)));
	}

	metrics
}

fn aggregate_metric_states(
	result_state: &str,
	metric_comparable: bool,
) -> BTreeMap<String, String> {
	let state = if metric_comparable { result_state } else { "not_encoded" };
	let mut states = BTreeMap::new();

	for k in QUANTITATIVE_K_VALUES {
		states.insert(format!("recall_at_{k}"), state.to_string());
		states.insert(format!("precision_at_{k}"), state.to_string());
		states.insert(format!("success_at_{k}"), state.to_string());
	}
	for metric in ["mrr", "ndcg_at_5", "average_precision"] {
		states.insert(metric.to_string(), state.to_string());
	}

	states
}

fn quantitative_metric_names() -> Vec<String> {
	let mut metrics = Vec::new();

	for k in QUANTITATIVE_K_VALUES {
		metrics.push(format!("recall_at_{k}"));
		metrics.push(format!("precision_at_{k}"));
		metrics.push(format!("success_at_{k}"));
	}
	for metric in ["mrr", "ndcg_at_5", "average_precision"] {
		metrics.push(metric.to_string());
	}

	metrics
}

fn per_query_denominators(
	candidate_count: usize,
	expected_relevant_count: usize,
) -> BTreeMap<String, usize> {
	let mut denominators = BTreeMap::new();

	for k in QUANTITATIVE_K_VALUES {
		denominators.insert(format!("recall_at_{k}"), expected_relevant_count);
		denominators.insert(format!("precision_at_{k}"), *k);
		denominators.insert(format!("success_at_{k}"), 1);
	}

	denominators.insert("mrr".to_string(), expected_relevant_count);
	denominators.insert("ndcg_at_5".to_string(), expected_relevant_count.min(5));
	denominators.insert("average_precision".to_string(), expected_relevant_count);
	denominators.insert("candidate_count".to_string(), candidate_count);

	denominators
}

fn aggregate_denominators(rows: &[QuantitativePerQueryRow]) -> BTreeMap<String, usize> {
	let mut denominators = BTreeMap::new();

	for k in QUANTITATIVE_K_VALUES {
		denominators.insert(
			format!("recall_at_{k}"),
			sum_per_query_denominator(rows, &format!("recall_at_{k}")),
		);
		denominators.insert(
			format!("precision_at_{k}"),
			sum_per_query_denominator(rows, &format!("precision_at_{k}")),
		);
		denominators.insert(
			format!("success_at_{k}"),
			sum_per_query_denominator(rows, &format!("success_at_{k}")),
		);
	}

	denominators.insert("mrr".to_string(), sum_per_query_denominator(rows, "mrr"));
	denominators.insert("ndcg_at_5".to_string(), sum_per_query_denominator(rows, "ndcg_at_5"));
	denominators.insert(
		"average_precision".to_string(),
		sum_per_query_denominator(rows, "average_precision"),
	);

	denominators
}

fn aggregate_confidence_intervals(
	rows: &[QuantitativePerQueryRow],
) -> BTreeMap<String, QuantitativeConfidenceInterval> {
	let mut confidence_intervals = BTreeMap::new();

	for metric in rate_metric_names() {
		let (numerator, denominator) = aggregate_rate_numerator_denominator(rows, metric.as_str());

		if denominator > 0 {
			confidence_intervals.insert(
				metric,
				wilson_confidence_interval(numerator.min(denominator), denominator),
			);
		}
	}

	confidence_intervals
}

fn rate_metric_names() -> Vec<String> {
	let mut metrics = Vec::new();

	for k in QUANTITATIVE_K_VALUES {
		metrics.push(format!("recall_at_{k}"));
		metrics.push(format!("precision_at_{k}"));
		metrics.push(format!("success_at_{k}"));
	}

	metrics
}

fn aggregate_rate_numerator_denominator(
	rows: &[QuantitativePerQueryRow],
	metric: &str,
) -> (usize, usize) {
	let mut numerator = 0;
	let mut denominator = 0;

	for row in rows {
		let Some(value) = row.metrics.get(metric).and_then(|value| *value) else {
			continue;
		};
		let Some(row_denominator) = row.denominators.get(metric).copied() else {
			continue;
		};

		if row_denominator == 0 {
			continue;
		}

		denominator += row_denominator;
		numerator += (value * row_denominator as f64).round() as usize;
	}

	(numerator, denominator)
}

fn wilson_confidence_interval(
	numerator: usize,
	denominator: usize,
) -> QuantitativeConfidenceInterval {
	let n = denominator as f64;
	let p = numerator as f64 / n;
	let z2 = WILSON_95_Z * WILSON_95_Z;
	let center = (p + z2 / (2.0 * n)) / (1.0 + z2 / n);
	let half_width =
		WILSON_95_Z * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()) / (1.0 + z2 / n);

	QuantitativeConfidenceInterval {
		method: "wilson_score".to_string(),
		confidence: 0.95,
		lower: formatting::round3((center - half_width).clamp(0.0, 1.0)),
		upper: formatting::round3((center + half_width).clamp(0.0, 1.0)),
		numerator,
		denominator,
	}
}

fn sum_per_query_denominator(rows: &[QuantitativePerQueryRow], metric: &str) -> usize {
	rows.iter().filter_map(|row| row.denominators.get(metric)).sum()
}

fn quantitative_corpus_id(source_jobs: &[RealWorldJob]) -> String {
	let ids = source_jobs.iter().map(|job| job.corpus.corpus_id.as_str()).collect::<BTreeSet<_>>();

	if ids.len() == 1 {
		ids.into_iter().next().unwrap_or("unknown").to_string()
	} else {
		"mixed".to_string()
	}
}

fn quantitative_suite_id(jobs: &[JobReport]) -> String {
	let suites = jobs.iter().map(|job| job.suite_id.as_str()).collect::<BTreeSet<_>>();

	if suites.len() == 1 {
		suites.into_iter().next().unwrap_or("unknown").to_string()
	} else {
		"mixed".to_string()
	}
}

fn quantitative_result_state(summary: &ReportSummary) -> &'static str {
	if summary.unsupported_claim > 0 {
		"unsupported_claim"
	} else if summary.wrong_result > 0 {
		"wrong_result"
	} else if summary.incomplete > 0 {
		"incomplete"
	} else if summary.blocked > 0 {
		"blocked"
	} else if summary.not_encoded > 0 {
		"not_encoded"
	} else {
		"pass"
	}
}

fn quantitative_evidence_class(adapter: &AdapterReport, jobs: &[JobReport]) -> &'static str {
	if adapter.behavior == "live_real_world_adapter" {
		"live_real_world"
	} else if jobs.iter().any(|job| job.operational_evidence_tier == "private_corpus") {
		"private_corpus"
	} else if jobs.iter().any(|job| job.operational_evidence_tier == "provider_backed") {
		"provider_backed"
	} else if adapter.behavior.contains("public_proxy") {
		"public_proxy"
	} else {
		"fixture_backed"
	}
}

fn quantitative_row_leaderboard_eligible(
	evidence_class: &str,
	sample_size: usize,
	ranking_query_count: usize,
	explicit_qrel_query_count: usize,
	metric_comparable: bool,
	audit_evidence: &QuantitativeAuditEvidence,
) -> bool {
	metric_comparable
		&& evidence_class == "live_real_world"
		&& sample_size >= MIN_LEADERBOARD_QUERY_COUNT
		&& ranking_query_count == sample_size
		&& explicit_qrel_query_count == ranking_query_count
		&& audit_evidence.held_out
		&& audit_evidence.leakage_audited
		&& audit_evidence
			.audit_manifest_id
			.as_deref()
			.is_some_and(|audit_manifest_id| !audit_manifest_id.trim().is_empty())
}

fn fixture_path_digest(path: &Path) -> Result<String> {
	let mut hasher = blake3::Hasher::new();

	if path.is_file() {
		hash_fixture_file(
			path,
			path.file_name().and_then(|name| name.to_str()).unwrap_or("fixture"),
			&mut hasher,
		)?;

		return Ok(hasher.finalize().to_hex().to_string());
	}

	let paths = audit_fixture_paths(path)?;

	for fixture in paths {
		let relative = fixture
			.strip_prefix(path)
			.map(|relative| relative.to_string_lossy().replace('\\', "/"))
			.unwrap_or_else(|_| fixture.to_string_lossy().replace('\\', "/"));

		hash_fixture_file(fixture.as_path(), relative.as_str(), &mut hasher)?;
	}

	Ok(hasher.finalize().to_hex().to_string())
}

fn audit_fixture_paths(path: &Path) -> Result<Vec<PathBuf>> {
	let mut paths = Vec::new();

	collect_audit_fixture_paths(path, &mut paths)?;

	paths.sort();

	Ok(paths)
}

fn collect_audit_fixture_paths(path: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
	if path.is_file() {
		paths.push(path.to_path_buf());

		return Ok(());
	}

	for entry in fs::read_dir(path)? {
		let entry_path = entry?.path();

		if entry_path.is_dir() {
			collect_audit_fixture_paths(entry_path.as_path(), paths)?;
		} else if entry_path.extension().and_then(|ext| ext.to_str()) == Some("json") {
			paths.push(entry_path);
		}
	}

	Ok(())
}

fn hash_fixture_file(path: &Path, logical_path: &str, hasher: &mut blake3::Hasher) -> Result<()> {
	hasher.update(logical_path.as_bytes());
	hasher.update(b"\0");
	hasher.update(&fs::read(path)?);
	hasher.update(b"\0");

	Ok(())
}

fn audit_artifact_display_path(path: &Path) -> String {
	let display_path = if path.is_absolute() {
		env::current_dir()
			.ok()
			.and_then(|cwd| path.strip_prefix(cwd).ok().map(Path::to_path_buf))
			.unwrap_or_else(|| path.to_path_buf())
	} else {
		path.to_path_buf()
	};

	display_path.to_string_lossy().replace('\\', "/")
}

fn ranking_query_ids(source_jobs: &[RealWorldJob]) -> BTreeSet<&str> {
	source_jobs
		.iter()
		.filter(|job| !ranking_relevance_grades(job).is_empty() && ranking_query_attempted(job))
		.map(|job| job.job_id.as_str())
		.collect()
}

fn ranking_query_count(source_jobs: &[RealWorldJob]) -> usize {
	ranking_query_ids(source_jobs).len()
}

fn explicit_qrel_query_count(source_jobs: &[RealWorldJob]) -> usize {
	source_jobs.iter().filter(|job| !job.expected_answer.relevance_judgments.is_empty()).count()
}

fn ranking_relevance_grades(source_job: &RealWorldJob) -> BTreeMap<String, f64> {
	if !source_job.expected_answer.relevance_judgments.is_empty() {
		return source_job
			.expected_answer
			.relevance_judgments
			.iter()
			.filter(|judgment| judgment.grade > 0.0)
			.map(|judgment| (judgment.evidence_id.clone(), judgment.grade))
			.collect();
	}

	source_job
		.required_evidence
		.iter()
		.filter(|evidence| matches!(evidence.requirement.as_str(), "cite" | "use" | "explain"))
		.map(|evidence| (evidence.evidence_id.clone(), 1.0))
		.collect()
}

fn ranking_query_attempted(job: &RealWorldJob) -> bool {
	if !scoring::produced_evidence_order(job).is_empty() {
		return true;
	}

	let Some(answer) = job.corpus.adapter_response.as_ref().map(|response| &response.answer) else {
		return false;
	};

	answer.trace_explainability.as_ref().is_some_and(|trace| {
		trace.stages.iter().any(|stage| stage.stage_name == "live_adapter.retrieve")
	}) && answer.latency_ms.is_some_and(|latency| latency.is_finite() && latency > 0.0)
}

fn qrel_source(source_job: &RealWorldJob, empty: bool) -> &'static str {
	if !source_job.expected_answer.relevance_judgments.is_empty() {
		"explicit_qrels"
	} else if empty {
		"not_encoded"
	} else {
		"expected_evidence_fallback"
	}
}

fn aggregate_qrel_source(
	ranking_query_count: usize,
	explicit_qrel_query_count: usize,
) -> &'static str {
	if ranking_query_count == 0 {
		"not_encoded"
	} else if explicit_qrel_query_count == ranking_query_count {
		"explicit_qrels"
	} else if explicit_qrel_query_count == 0 {
		"expected_evidence_fallback"
	} else {
		"mixed"
	}
}

fn ranking_coverage_state(
	summary: &ReportSummary,
	source_job_count: usize,
	ranking_query_count: usize,
) -> &'static str {
	if ranking_query_count == 0 {
		"not_encoded"
	} else if ranking_query_count == source_job_count && summary.not_encoded == 0 {
		"complete"
	} else {
		"partial_coverage"
	}
}

fn ranked_candidate_source(ranking_query_count: usize) -> &'static str {
	if ranking_query_count == 0 { "not_encoded" } else { "produced_evidence_order" }
}

fn positive_qrel_count(relevance: &BTreeMap<String, f64>) -> usize {
	relevance.values().filter(|grade| **grade > 0.0).count()
}

fn rate(numerator: usize, denominator: usize) -> Option<f64> {
	(denominator > 0).then(|| formatting::round3(numerator as f64 / denominator as f64))
}
