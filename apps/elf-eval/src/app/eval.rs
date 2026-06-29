use std::{path::Path, time::Instant};

use color_eyre::{Result, eyre};
use uuid::Uuid;

use crate::app::{
	Args, SearchMode, dataset,
	metrics::{self},
	types::{
		EvalDataset, EvalDatasetInfo, EvalRun, EvalSettings, ExpectedKind, QueryReport,
		QueryStability, StabilitySummary, default_eval_defaults,
	},
};
use elf_config::Config;
use elf_service::{ElfService, SearchIndexResponse, SearchRequest};
use elf_storage::{db::Db, qdrant::QdrantStore};

pub(super) async fn eval_config(
	config_path: &Path,
	config: Config,
	dataset: &EvalDataset,
	args: &Args,
	search_mode: SearchMode,
) -> Result<EvalRun> {
	let db = Db::connect(&config.storage.postgres).await?;

	db.ensure_schema(config.storage.qdrant.vector_dim).await?;

	let qdrant = QdrantStore::new(&config.storage.qdrant)?;
	let service = ElfService::new(config, db, qdrant);
	let defaults = dataset.defaults.clone().unwrap_or_else(default_eval_defaults);
	let runs_per_query = args.runs_per_query.max(1);
	let mut reports = Vec::with_capacity(dataset.queries.len());
	let mut latencies_ms = Vec::with_capacity(dataset.queries.len());
	let mut stability_positional = Vec::new();
	let mut stability_set = Vec::new();

	for (index, query) in dataset.queries.iter().enumerate() {
		let merged = dataset::merge_query(&defaults, query, args, &service.cfg, index)?;
		let (first, latency_ms, stability, trace_ids) =
			run_query_n_times(&service, merged.request.clone(), runs_per_query, search_mode)
				.await?;
		let retrieved = metrics::unique_items(&first.items);
		let retrieved_note_ids: Vec<Uuid> = retrieved.iter().map(|item| item.note_id).collect();
		let retrieved_keys: Vec<Option<String>> =
			retrieved.iter().map(|item| item.key.clone()).collect();
		let retrieved_summary_chars =
			retrieved.iter().map(|item| item.summary.len()).sum::<usize>();
		let (metrics, expected_count) =
			metrics::compute_metrics_for_query(&merged, &retrieved_note_ids, &retrieved_keys);

		if let Some(s) = stability {
			stability_positional.push(s.positional_churn_at_k);
			stability_set.push(s.set_churn_at_k);
		}

		reports.push(QueryReport {
			id: merged.id,
			query: merged.query,
			trace_id: first.trace_id,
			trace_ids: (trace_ids.len() > 1).then_some(trace_ids),
			expected_count,
			retrieved_count: retrieved_note_ids.len(),
			relevant_count: metrics.relevant_count,
			recall_at_k: metrics.recall_at_k,
			precision_at_k: metrics.precision_at_k,
			rr: metrics.rr,
			ndcg: metrics.ndcg,
			latency_ms,
			expected_note_ids: merged.expected_note_ids,
			expected_keys: merged.expected_keys,
			expected_kind: merged.expected_kind,
			retrieved_note_ids,
			retrieved_keys: if merged.expected_kind == ExpectedKind::Key {
				retrieved_keys
			} else {
				Vec::new()
			},
			retrieved_summary_chars,
			stability,
		});
		latencies_ms.push(latency_ms);
	}

	let mut summary = metrics::summarize(&reports, &latencies_ms);

	if runs_per_query > 1 && !stability_positional.is_empty() {
		let count = stability_positional.len().max(1) as f64;
		let avg_positional_churn_at_k = stability_positional.iter().sum::<f64>() / count;
		let avg_set_churn_at_k = stability_set.iter().sum::<f64>() / count;

		summary.stability = Some(StabilitySummary {
			runs_per_query,
			avg_positional_churn_at_k,
			avg_set_churn_at_k,
		});
	}

	let settings = EvalSettings {
		config_path: config_path.display().to_string(),
		search_mode,
		candidate_k: args
			.candidate_k
			.or(dataset.defaults.as_ref().and_then(|d| d.candidate_k))
			.unwrap_or(service.cfg.memory.candidate_k),
		top_k: args
			.top_k
			.or(dataset.defaults.as_ref().and_then(|d| d.top_k))
			.unwrap_or(service.cfg.memory.top_k),
		runs_per_query: (runs_per_query > 1).then_some(runs_per_query),
	};

	Ok(EvalRun {
		dataset: EvalDatasetInfo {
			name: dataset.name.clone().unwrap_or_else(|| "eval".to_string()),
			query_count: reports.len(),
		},
		settings,
		summary,
		queries: reports,
	})
}

async fn run_query_n_times(
	service: &ElfService,
	request: SearchRequest,
	runs_per_query: u32,
	search_mode: SearchMode,
) -> Result<(SearchIndexResponse, f64, Option<QueryStability>, Vec<Uuid>)> {
	let k = request.top_k.unwrap_or(1).max(1) as usize;
	let runs = runs_per_query.max(1);
	let mut first_response: Option<SearchIndexResponse> = None;
	let mut first_retrieved_ids: Vec<Uuid> = Vec::new();
	let mut trace_ids: Vec<Uuid> = Vec::with_capacity(runs as usize);
	let mut latency_total_ms = 0.0_f64;
	let mut positional_churn_sum = 0.0_f64;
	let mut set_churn_sum = 0.0_f64;
	let mut churn_count = 0_u32;

	for run_idx in 0..runs {
		let start = Instant::now();
		let response = search_with_mode(service, request.clone(), search_mode).await?;
		let latency_ms = start.elapsed().as_secs_f64() * 1_000.0;

		latency_total_ms += latency_ms;

		trace_ids.push(response.trace_id);

		let retrieved = metrics::unique_items(&response.items);
		let retrieved_ids = retrieved.iter().map(|item| item.note_id).collect::<Vec<_>>();

		if run_idx == 0 {
			first_retrieved_ids = retrieved_ids;
			first_response = Some(response);

			continue;
		}

		let (positional_churn_at_k, set_churn_at_k) =
			metrics::churn_against_baseline_at_k(&first_retrieved_ids, &retrieved_ids, k);

		positional_churn_sum += positional_churn_at_k;
		set_churn_sum += set_churn_at_k;
		churn_count += 1;
	}

	let latency_ms_mean = latency_total_ms / runs as f64;
	let stability = if churn_count > 0 {
		Some(QueryStability {
			runs_per_query: runs,
			positional_churn_at_k: positional_churn_sum / churn_count as f64,
			set_churn_at_k: set_churn_sum / churn_count as f64,
		})
	} else {
		None
	};

	Ok((
		first_response.ok_or_else(|| eyre::eyre!("No search responses were collected."))?,
		latency_ms_mean,
		stability,
		trace_ids,
	))
}

async fn search_with_mode(
	service: &ElfService,
	request: SearchRequest,
	search_mode: SearchMode,
) -> Result<SearchIndexResponse> {
	match search_mode {
		SearchMode::QuickFind => service.search_quick(request).await.map_err(|err| err.into()),
		SearchMode::PlannedSearch => {
			let response = service.search_planned(request).await?;

			Ok(SearchIndexResponse {
				trace_id: response.trace_id,
				search_session_id: response.search_session_id,
				expires_at: response.expires_at,
				items: response.items,
				trajectory_summary: response.trajectory_summary,
			})
		},
	}
}
