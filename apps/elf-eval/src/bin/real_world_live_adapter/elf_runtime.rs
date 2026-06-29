use super::*;

pub(super) async fn run_elf(args: ElfArgs) -> color_eyre::Result<()> {
	let jobs = load_jobs(&args.fixtures)?;
	let result = materialize_elf_jobs(&args, &jobs).await;
	let materialized = match result {
		Ok(jobs) => jobs,
		Err(err) => failure_jobs(&args.adapter_id, &jobs, "elf_service_runtime", err.to_string()),
	};

	write_materialized_output(MaterializedOutput {
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

async fn materialize_elf_jobs(
	args: &ElfArgs,
	jobs: &[LoadedJob],
) -> color_eyre::Result<Vec<MaterializedJob>> {
	let base_dsn = env::var("ELF_PG_DSN")
		.map_err(|_| eyre::eyre!("ELF_PG_DSN must be set for ELF live real-world adapter."))?;
	let qdrant_url = env::var("ELF_QDRANT_GRPC_URL")
		.or_else(|_| env::var("ELF_QDRANT_URL"))
		.map_err(|_| eyre::eyre!("ELF_QDRANT_GRPC_URL or ELF_QDRANT_URL must be set."))?;
	let test_db = TestDatabase::new(&base_dsn).await?;
	let run_suffix = short_hash(format!("{}:{}", args.adapter_id, Uuid::new_v4()).as_str());
	let runtime = BaselineRuntime {
		config_path: args.config.clone(),
		dsn: test_db.dsn().to_string(),
		qdrant_url,
		collection: format!("elf_live_real_world_{run_suffix}"),
		docs_collection: format!("elf_live_real_world_docs_{run_suffix}"),
	};
	let service = build_service(&runtime).await?;
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
) -> color_eyre::Result<MaterializedJob> {
	if let Some(job) = declared_encoding_job(adapter_id, loaded) {
		return Ok(job);
	}
	if let Some(job) = not_encoded_job(adapter_id, loaded) {
		return Ok(job);
	}

	let corpus = corpus_texts(loaded)?;
	let stored_corpus = elf_stored_corpus_texts(&corpus)?;
	let project_id = project_id_for_job(&loaded.job.job_id);
	let ingested =
		ingest_elf_corpus(service, loaded, adapter_id, project_id.as_str(), &corpus).await?;

	run_worker(runtime).await?;

	let (response, latency_ms) = search_elf_job(service, loaded, &project_id).await?;
	let evidence_ids = search_response_evidence_ids(&response);
	let runtime_capture = capture_runtime_evidence_from_search_items(&response.items);
	let capture = capture_with_runtime_source_refs(ingested.capture.clone(), &runtime_capture);
	let capture_failure = validate_capture_runtime_evidence(
		loaded.job.suite.as_str(),
		&corpus,
		&capture,
		&runtime_capture,
	);
	let (selected, temporal_reconciliation, trace_stages) = elf_selected_evidence_text(
		loaded,
		&stored_corpus,
		&evidence_ids,
		&ingested,
		&capture_failure,
	);
	let replay_command = elf_replay_command(response.trace_id, project_id.as_str());
	let (operator_debug, operator_debug_evidence) = operator_debug_output(
		AdapterKind::ElfServiceRuntime,
		loaded,
		Some(response.trace_id),
		replay_command,
		format!(
			"/v2/admin/traces/{}/bundle?mode=full&stage_items_limit=128&candidates_limit=200",
			response.trace_id
		),
	);
	let (pages, knowledge, knowledge_failure) =
		match materialize_elf_knowledge(service, loaded, &ingested, adapter_id).await {
			Ok(output) => output,
			Err(err) if loaded.job.suite == "knowledge_compilation" =>
				(Vec::new(), None, Some(format!("live_adapter.knowledge: {err}"))),
			Err(_) => (Vec::new(), None, None),
		};
	let (consolidation_response, consolidation, consolidation_failure) =
		match materialize_elf_consolidation(runtime, service, loaded, &ingested, adapter_id).await {
			Ok(output) => output,
			Err(err) if loaded.job.suite == "consolidation" =>
				(None, None, Some(format!("live_adapter.consolidation: {err}"))),
			Err(_) => (None, None, None),
		};
	let dreaming_readback = materialize_elf_dreaming_readback(
		service,
		loaded,
		project_id.as_str(),
		response.trace_id,
		adapter_id,
	)
	.await?;
	let dreaming_failure = dreaming_readback.as_ref().and_then(|output| {
		if output.materialization.missing_source_refs.is_empty() {
			None
		} else {
			Some(format!(
				"live_adapter.dreaming_readback missing source refs: {}",
				output.materialization.missing_source_refs.join(", ")
			))
		}
	});
	let failure = knowledge_failure.or(consolidation_failure).or(dreaming_failure);
	let suite_selection = suite_materialization_selection(SuiteMaterializationSelectionInput {
		loaded,
		ingested: &ingested,
		capture_failure: &capture_failure,
		selected,
		trace_stages,
		knowledge: &knowledge,
		consolidation: &consolidation,
		dreaming_readback,
	});

	Ok(materialized_job(
		loaded,
		adapter_id,
		MaterializedJobInput {
			content: suite_selection.selected.content,
			evidence_ids: suite_selection.selected.evidence_ids,
			pages,
			latency_ms,
			indexing_latency_ms: None,
			returned_count: response.items.len(),
			trace_id: Some(response.trace_id),
			failure,
			source_mappings: Vec::new(),
			operator_debug,
			operator_debug_evidence,
			capture: capture_for_job(loaded, capture),
			capture_failure,
			consolidation_response,
			consolidation,
			knowledge,
			temporal_reconciliation,
			dreaming_readback: suite_selection.dreaming_readback,
			memory_summaries: suite_selection.memory_summaries,
			proactive_briefs: suite_selection.proactive_briefs,
			scheduled_tasks: suite_selection.scheduled_tasks,
			trace_stages: suite_selection.trace_stages,
		},
	))
}

async fn search_elf_job(
	service: &ElfService,
	loaded: &LoadedJob,
	project_id: &str,
) -> color_eyre::Result<(SearchResponse, f64)> {
	let started_at = Instant::now();
	let response = service
		.search_raw(SearchRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: project_id.to_string(),
			agent_id: AGENT_ID.to_string(),
			token_id: None,
			payload_level: PayloadLevel::L2,
			read_profile: "private_only".to_string(),
			query: loaded.job.prompt.content.clone(),
			top_k: Some(5),
			candidate_k: Some(20),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.map_err(|err| eyre::eyre!("ELF search_raw failed for {}: {err}", loaded.job.job_id))?;

	Ok((response, started_at.elapsed().as_secs_f64() * 1_000.0))
}
