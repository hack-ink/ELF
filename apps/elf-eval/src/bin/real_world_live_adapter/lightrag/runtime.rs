use std::{
	fs,
	time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

use crate::{
	AdapterKind, CommandEvidence, LightragArgs, LoadedJob, MaterializedJob, MaterializedJobInput,
	MaterializedOutput, Result, Uuid, eyre,
	lightrag::{api, corpus, mapping, metadata, status},
};

pub(crate) async fn run_lightrag_async(args: LightragArgs) -> Result<()> {
	let jobs = crate::load_jobs(&args.fixtures)?;
	let result = materialize_lightrag_jobs(&args, &jobs).await;
	let materialized = match result {
		Ok(jobs) => jobs,
		Err(err) => status::lightrag_failure_jobs(
			&args.adapter_id,
			&jobs,
			"lightrag_api_context_export",
			err.to_string(),
		),
	};
	let status = crate::aggregate_status(&materialized);

	crate::write_materialized_output(MaterializedOutput {
		adapter_id: &args.adapter_id,
		adapter_kind: AdapterKind::LightragApiContextExport,
		fixtures: &args.fixtures,
		out_fixtures: &args.out_fixtures,
		evidence_out: &args.evidence_out,
		jobs: &jobs,
		materialized: &materialized,
		command_evidence: vec![CommandEvidence {
			label: "lightrag_api_context_export".to_string(),
			status,
			command: "cargo run -p elf-eval --bin real_world_live_adapter -- lightrag"
				.to_string(),
			artifact: Some(args.evidence_out.display().to_string()),
			reason: "LightRAG adapter used /documents/texts, /documents/track_status, and /query with only_need_context plus chunk references.".to_string(),
		}],
		metadata: Some(metadata::lightrag_metadata(&args)),
	})
}

#[derive(Deserialize, Serialize)]
struct LightragIndexState {
	schema: String,
	run_slug: String,
	job_ids: Vec<String>,
}

async fn materialize_lightrag_jobs(
	args: &LightragArgs,
	jobs: &[LoadedJob],
) -> Result<Vec<MaterializedJob>> {
	fs::create_dir_all(&args.work_dir)?;
	let state_path = args.work_dir.join("index-state.json");
	let expected_job_ids = jobs.iter().map(|job| job.job.job_id.clone()).collect::<Vec<_>>();
	let state = if args.reuse_index {
		let state: LightragIndexState =
			serde_json::from_slice(&fs::read(&state_path).map_err(|err| {
				eyre::eyre!("LightRAG warm state is missing at {}: {err}", state_path.display())
			})?)?;
		if state.job_ids != expected_job_ids {
			return Err(eyre::eyre!("LightRAG warm state job identity differs from cold ingest."));
		}
		state
	} else {
		LightragIndexState {
			schema: "elf.lightrag_index_state/v1".to_string(),
			run_slug: crate::short_hash(format!("{}:{}", args.adapter_id, Uuid::new_v4()).as_str()),
			job_ids: expected_job_ids,
		}
	};

	let client = reqwest::Client::builder().timeout(Duration::from_secs(180)).build()?;

	api::wait_for_lightrag(args, &client).await?;

	let mut out = Vec::with_capacity(jobs.len());

	for loaded in jobs {
		out.push(materialize_lightrag_job(args, &client, loaded, &state.run_slug).await?);
	}
	if !args.reuse_index {
		fs::write(&state_path, serde_json::to_vec_pretty(&state)?)?;
	}

	Ok(out)
}

async fn materialize_lightrag_job(
	args: &LightragArgs,
	client: &reqwest::Client,
	loaded: &LoadedJob,
	run_slug: &str,
) -> Result<MaterializedJob> {
	if let Some(job) = crate::declared_encoding_job(&args.adapter_id, loaded) {
		return Ok(job);
	}
	if let Some(job) = status::lightrag_not_encoded_job(&args.adapter_id, loaded) {
		return Ok(job);
	}

	let corpus = crate::corpus_texts(loaded)?;
	let sources = corpus::write_lightrag_corpus(args, loaded, &corpus, run_slug)?;
	let phase = if args.reuse_index { "warm" } else { "cold" };
	let native_dir = args.work_dir.join("native").join(phase);
	fs::create_dir_all(&native_dir)?;
	let indexing_latency_ms = if args.reuse_index {
		None
	} else {
		let indexed_at = Instant::now();
		let insert_response = api::insert_lightrag_texts(args, client, &corpus, &sources).await?;
		fs::write(
			native_dir.join(format!("{}-insert.json", crate::slug(&loaded.job.job_id))),
			serde_json::to_vec_pretty(&insert_response)?,
		)?;
		api::wait_for_lightrag_index(args, client, &insert_response, corpus.len()).await?;
		Some(indexed_at.elapsed().as_secs_f64() * 1_000.0)
	};
	let queried_at = Instant::now();
	let query_response = api::query_lightrag_context(args, client, loaded).await?;
	let latency_ms = queried_at.elapsed().as_secs_f64() * 1_000.0;
	fs::write(
		native_dir.join(format!("{}-query.json", crate::slug(&loaded.job.job_id))),
		serde_json::to_vec_pretty(&query_response)?,
	)?;
	let references = query_response
		.get("references")
		.and_then(serde_json::Value::as_array)
		.ok_or_else(|| eyre::eyre!("LightRAG query response omitted native references."))?;
	let source_mappings = mapping::lightrag_source_mappings(&sources, &query_response);
	if !references.is_empty()
		&& source_mappings.iter().any(|mapping| mapping.evidence_ids.len() != 1)
	{
		return Err(eyre::eyre!(
			"LightRAG returned a reference that could not map to one native source identity."
		));
	}
	let evidence_ids = mapping::lightrag_mapped_evidence_ids(&source_mappings);
	let selected = crate::selected_retrieved_corpus_texts(&corpus, &evidence_ids);

	Ok(crate::materialized_job(
		loaded,
		&args.adapter_id,
		MaterializedJobInput {
			content: selected.content,
			evidence_ids: selected.evidence_ids,
			pages: Vec::new(),
			latency_ms,
			indexing_latency_ms,
			returned_count: source_mappings.len(),
			trace_id: None,
			failure: None,
			source_mappings,
			operator_debug: None,
			operator_debug_evidence: None,
			capture: None,
			capture_failure: None,
			consolidation_response: None,
			consolidation: None,
			knowledge: None,
			temporal_reconciliation: None,
			dreaming_readback: None,
			memory_summaries: Vec::new(),
			proactive_briefs: Vec::new(),
			scheduled_tasks: Vec::new(),
			trace_stages: None,
		},
	))
}
