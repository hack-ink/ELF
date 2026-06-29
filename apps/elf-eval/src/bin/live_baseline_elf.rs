#![allow(clippy::single_component_path_imports, unused_crate_dependencies)]

//! Docker live-baseline runner for ELF's own same-corpus retrieval path.

#[path = "live_baseline_elf/backfill.rs"] mod backfill;
#[path = "live_baseline_elf/checks.rs"] mod checks;
#[path = "live_baseline_elf/corpus.rs"] mod corpus;
#[path = "live_baseline_elf/providers.rs"] mod providers;
#[path = "live_baseline_elf/runtime.rs"] mod runtime;
#[path = "live_baseline_elf/types.rs"] mod types;

use std::{
	collections::{BTreeMap, HashSet},
	env, fs,
	path::{Path, PathBuf},
	process::Command,
	sync::Arc,
	time::{Duration, Instant},
};

use blake3::Hasher;
use clap::Parser;
use color_eyre::{Report, Result, eyre};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{task::JoinSet, time};
use uuid::Uuid;

use backfill::worker_concurrency;
use checks::{outbox_done, parse_env_usize};
use corpus::{
	contains_case_insensitive, distinctive_terms, embed_text, evidence_id_for_doc,
	expected_docs_for_case, key_for_doc, terms,
};
use elf_chunking::ChunkingConfig;
use elf_config::{Config, EmbeddingProviderConfig, LlmProviderConfig, ProviderConfig};
use elf_service::{
	AddNoteInput, AddNoteRequest, BoxFuture, DeleteRequest, ElfService, EmbeddingProvider,
	ExtractorProvider, NoteOp, PayloadLevel, Providers, RerankProvider, SearchRequest,
	UpdateRequest,
};
use elf_storage::{db::Db, qdrant::QdrantStore};
use elf_testkit::TestDatabase;
use elf_worker::worker::{self, WorkerState};
use providers::{
	EmbeddingMode, deterministic_providers, embedding_mode, env_string, runtime_config,
};
use runtime::run_single_query;
use types::{
	Args, BackfillAttemptEvidence, BackfillCheckpoint, BackfillCheckpointEntry, BackfillOutcome,
	BackfillReport, BackfillResumeReport, BaselineRuntime, CheckResult, CheckSummary, CorpusNote,
	CostProxyReport, DuplicateSourceNote, ElfBaselineReport, EmbeddingRuntimeReport,
	ExistingBackfillNote, FailedOutboxJob, IndexingReport, OperationalCase, QueryCase,
	QueryManifest, QueryResult, QuerySummary, ResourceEnvelopeEvidence, SoakConfig,
	WorkerRunEvidence,
};

const TENANT_ID: &str = "elf-live-baseline";
const PROJECT_ID: &str = "shared-corpus";
const AGENT_ID: &str = "elf-bench-agent";
const SCOPE: &str = "agent_private";
const BACKFILL_CHECKPOINT_SCHEMA: &str = "elf.live_baseline.backfill_checkpoint/v1";

fn report_reason(status: &str, check_summary: &CheckSummary) -> String {
	if status == "pass" {
		"ELF added the corpus, rebuilt Qdrant, and returned expected evidence for every query"
			.to_string()
	} else {
		format!(
			"ELF reported {} wrong-result, {} lifecycle-failure, {} blocked, {} incomplete, and {} not-encoded live-baseline check(s)",
			check_summary.wrong_result,
			check_summary.lifecycle_fail,
			check_summary.blocked,
			check_summary.incomplete,
			check_summary.not_encoded
		)
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;

	let args = Args::parse();
	let out = args.out.clone();
	let report = run(args).await?;
	let raw = serde_json::to_string_pretty(&report)?;

	fs::write(out, raw)?;

	Ok(())
}

async fn run(args: Args) -> Result<ElfBaselineReport> {
	let started_at = Instant::now();
	let base_dsn = env::var("ELF_PG_DSN")
		.map_err(|_| eyre::eyre!("ELF_PG_DSN must be set for live ELF baseline."))?;
	let qdrant_url = env::var("ELF_QDRANT_GRPC_URL")
		.or_else(|_| env::var("ELF_QDRANT_URL"))
		.map_err(|_| eyre::eyre!("ELF_QDRANT_GRPC_URL or ELF_QDRANT_URL must be set."))?;
	let test_db = TestDatabase::new(&base_dsn).await?;
	let collection = test_db.collection_name("elf_live_baseline_notes");
	let docs_collection = test_db.collection_name("elf_live_baseline_docs");
	let runtime = BaselineRuntime {
		config_path: args.config.clone(),
		dsn: test_db.dsn().to_string(),
		qdrant_url,
		collection,
		docs_collection,
	};
	let service = Arc::new(runtime::build_service(&runtime).await?);
	let notes = corpus::load_corpus_notes(&args.corpus)?;
	let backfill_checkpoint_path = backfill::backfill_checkpoint_path(&args.out);
	let backfill =
		backfill::run_resumable_backfill(&service, &notes, &backfill_checkpoint_path).await?;
	let note_ids = backfill.note_ids;
	let initial_worker =
		runtime::run_worker_until_indexed(&runtime, &service, &note_ids, "corpus_upsert").await?;
	let rebuild = service.rebuild_qdrant().await?;
	let query_manifest = corpus::load_queries(&args.queries)?;
	let query_results = runtime::run_queries(&service, query_manifest.queries).await?;
	let pass_count = query_results.iter().filter(|result| result.matched).count();
	let fail_count = query_results.len().saturating_sub(pass_count);
	let latency_ms_total = query_results.iter().map(|result| result.latency_ms).sum::<f64>();
	let latency_ms_mean = latency_ms_total / query_results.len().max(1) as f64;
	let latency_values = query_results.iter().map(|result| result.latency_ms).collect::<Vec<_>>();
	let latency_ms_p50 = checks::latency_percentile(&latency_values, 0.50);
	let latency_ms_p95 = checks::latency_percentile(&latency_values, 0.95);
	let latency_ms_p99 = checks::latency_percentile(&latency_values, 0.99);
	let latency_ms_max = latency_values.iter().copied().fold(0.0_f64, f64::max);
	let retrieval_status =
		if fail_count == 0 { "retrieval_pass" } else { "retrieval_wrong_result" };
	let mut checks = vec![
		checks::resumable_backfill_check(&backfill.report),
		checks::retrieval_check(&query_results),
		checks::worker_indexing_check(initial_worker),
	];

	checks.extend(checks::run_lifecycle_checks(&runtime, &service, &notes, &note_ids).await?);
	checks.push(checks::run_concurrent_write_check(&runtime, Arc::clone(&service)).await?);

	if let Some(soak_check) =
		checks::run_soak_stability_check(&runtime, Arc::clone(&service)).await?
	{
		checks.push(soak_check);
	}

	checks.push(
		checks::resource_envelope_check(
			&service,
			&args.corpus,
			&args.out,
			&backfill_checkpoint_path,
			started_at.elapsed().as_secs_f64(),
		)
		.await,
	);

	let check_summary = checks::summarize_checks(&checks);
	let status = checks::project_status_from_summary(&check_summary);
	let reason = report_reason(status, &check_summary);
	let embedding = providers::embedding_runtime_report(&service.cfg);
	let cost_proxy = checks::cost_proxy_report(&notes, &query_results, &embedding);
	let report = ElfBaselineReport {
		schema: "elf.live_baseline.elf_result/v1",
		status,
		retrieval_status,
		reason,
		head: corpus::git_head().unwrap_or_else(|_| "unknown".to_string()),
		embedding,
		cost_proxy,
		backfill: backfill.report,
		indexing: IndexingReport {
			note_count: notes.len(),
			rebuild_rebuilt_count: rebuild.rebuilt_count,
			rebuild_missing_vector_count: rebuild.missing_vector_count,
			rebuild_error_count: rebuild.error_count,
		},
		summary: QuerySummary {
			total: query_results.len(),
			pass: pass_count,
			fail: fail_count,
			wrong_result_count: fail_count,
			latency_ms_total,
			latency_ms_mean,
			latency_ms_p50,
			latency_ms_p95,
			latency_ms_p99,
			latency_ms_max,
		},
		check_summary,
		checks,
		queries: query_results,
		ops_cases: checks::operational_cases(),
	};

	drop(service);

	test_db.cleanup().await?;

	Ok(report)
}
