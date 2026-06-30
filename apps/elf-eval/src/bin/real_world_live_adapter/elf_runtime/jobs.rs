use crate::{
	AdapterKind, BaselineRuntime, CommandEvidence, ElfArgs, ElfService, LoadedJob, MaterializedJob,
	MaterializedJobInput, MaterializedOutput, Result, SuiteMaterializationSelectionInput,
	TestDatabase, Uuid, aggregate_status,
	elf_runtime::{search, surfaces},
	env, eyre,
};

pub(crate) async fn run_elf(args: ElfArgs) -> Result<()> {
	let jobs = crate::load_jobs(&args.fixtures)?;
	let result = materialize_elf_jobs(&args, &jobs).await;
	let materialized = match result {
		Ok(jobs) => jobs,
		Err(err) =>
			crate::failure_jobs(&args.adapter_id, &jobs, "elf_service_runtime", err.to_string()),
	};

	crate::write_materialized_output(MaterializedOutput {
		adapter_id: &args.adapter_id,
		adapter_kind: AdapterKind::ElfServiceRuntime,
		fixtures: &args.fixtures,
		out_fixtures: &args.out_fixtures,
		evidence_out: &args.evidence_out,
		jobs: &jobs,
		materialized: &materialized,
		command_evidence: vec![CommandEvidence {
			label: "elf_service_runtime".to_string(),
			status: aggregate_status(&materialized),
			command: "cargo run -p elf-eval --bin real_world_live_adapter -- elf".to_string(),
			artifact: Some(args.evidence_out.display().to_string()),
			reason: "ELF live adapter used ElfService, worker indexing, and search_raw."
				.to_string(),
		}],
		metadata: None,
	})
}

async fn materialize_elf_jobs(args: &ElfArgs, jobs: &[LoadedJob]) -> Result<Vec<MaterializedJob>> {
	let base_dsn = env::var("ELF_PG_DSN")
		.map_err(|_| eyre::eyre!("ELF_PG_DSN must be set for ELF live real-world adapter."))?;
	let qdrant_url = env::var("ELF_QDRANT_GRPC_URL")
		.or_else(|_| env::var("ELF_QDRANT_URL"))
		.map_err(|_| eyre::eyre!("ELF_QDRANT_GRPC_URL or ELF_QDRANT_URL must be set."))?;
	let test_db = TestDatabase::new(&base_dsn).await?;
	let run_suffix = crate::short_hash(format!("{}:{}", args.adapter_id, Uuid::new_v4()).as_str());
	let runtime = BaselineRuntime {
		config_path: args.config.clone(),
		dsn: test_db.dsn().to_string(),
		qdrant_url,
		collection: format!("elf_live_real_world_{run_suffix}"),
		docs_collection: format!("elf_live_real_world_docs_{run_suffix}"),
	};
	let service = crate::build_service(&runtime).await?;
	let mut out = Vec::with_capacity(jobs.len());

	for loaded in jobs {
		out.push(materialize_elf_job(&runtime, &service, loaded, &args.adapter_id).await?);
	}

	drop(service);

	test_db.cleanup().await?;

	Ok(out)
}

async fn materialize_elf_job(
	runtime: &BaselineRuntime,
	service: &ElfService,
	loaded: &LoadedJob,
	adapter_id: &str,
) -> Result<MaterializedJob> {
	if let Some(job) = crate::declared_encoding_job(adapter_id, loaded) {
		return Ok(job);
	}
	if let Some(job) = crate::not_encoded_job(adapter_id, loaded) {
		return Ok(job);
	}

	let corpus = crate::corpus_texts(loaded)?;
	let stored_corpus = crate::elf_stored_corpus_texts(&corpus)?;
	let project_id = crate::project_id_for_job(&loaded.job.job_id);
	let ingested =
		crate::ingest_elf_corpus(service, loaded, adapter_id, project_id.as_str(), &corpus).await?;

	crate::run_worker(runtime).await?;

	let (response, latency_ms) = search::search_elf_job(service, loaded, &project_id).await?;
	let evidence_ids = crate::search_response_evidence_ids(&response);
	let runtime_capture = crate::capture_runtime_evidence_from_search_items(&response.items);
	let capture =
		crate::capture_with_runtime_source_refs(ingested.capture.clone(), &runtime_capture);
	let capture_failure = crate::validate_capture_runtime_evidence(
		loaded.job.suite.as_str(),
		&corpus,
		&capture,
		&runtime_capture,
	);
	let (selected, temporal_reconciliation, trace_stages) = crate::elf_selected_evidence_text(
		loaded,
		&stored_corpus,
		&evidence_ids,
		&ingested,
		&capture_failure,
	);
	let replay_command = crate::elf_replay_command(response.trace_id, project_id.as_str());
	let (operator_debug, operator_debug_evidence) = crate::operator_debug_output(
		AdapterKind::ElfServiceRuntime,
		loaded,
		Some(response.trace_id),
		replay_command,
		format!(
			"/v2/admin/traces/{}/bundle?mode=full&stage_items_limit=128&candidates_limit=200",
			response.trace_id
		),
	);
	let optional = surfaces::materialize_optional_elf_surfaces(
		runtime,
		service,
		loaded,
		&ingested,
		project_id.as_str(),
		response.trace_id,
		adapter_id,
	)
	.await?;
	let suite_selection =
		crate::suite_materialization_selection(SuiteMaterializationSelectionInput {
			loaded,
			ingested: &ingested,
			capture_failure: &capture_failure,
			selected,
			trace_stages,
			knowledge: &optional.knowledge,
			consolidation: &optional.consolidation,
			dreaming_readback: optional.dreaming_readback,
		});

	Ok(crate::materialized_job(
		loaded,
		adapter_id,
		MaterializedJobInput {
			content: suite_selection.selected.content,
			evidence_ids: suite_selection.selected.evidence_ids,
			pages: optional.pages,
			latency_ms,
			indexing_latency_ms: None,
			returned_count: response.items.len(),
			trace_id: Some(response.trace_id),
			failure: optional.failure,
			source_mappings: Vec::new(),
			operator_debug,
			operator_debug_evidence,
			capture: crate::capture_for_job(loaded, capture),
			capture_failure,
			consolidation_response: optional.consolidation_response,
			consolidation: optional.consolidation,
			knowledge: optional.knowledge,
			temporal_reconciliation,
			dreaming_readback: suite_selection.dreaming_readback,
			memory_summaries: suite_selection.memory_summaries,
			proactive_briefs: suite_selection.proactive_briefs,
			scheduled_tasks: suite_selection.scheduled_tasks,
			trace_stages: suite_selection.trace_stages,
		},
	))
}
