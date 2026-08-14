use crate::{
	AGENT_ID, AdapterKind, BaselineRuntime, CommandEvidence, CorpusText, DeleteRequest, ElfArgs,
	ElfService, IngestedCorpus, LoadedJob, MaterializedJob, MaterializedJobInput,
	MaterializedOutput, NoteOp, Result, SuiteMaterializationSelectionInput, TENANT_ID,
	UpdateRequest, aggregate_status,
	elf_runtime::{search, surfaces},
	env, eyre, fs, serde_json,
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
		metadata: Some(serde_json::json!({"index_reused": args.reuse_index})),
	})
}

fn elf_job_content(
	loaded: &LoadedJob,
	contexts: &[serde_json::Value],
	selected_content: String,
) -> String {
	if loaded.job.operations.is_empty() {
		selected_content
	} else {
		contexts
			.iter()
			.filter_map(|context| context.get("text").and_then(serde_json::Value::as_str))
			.collect::<Vec<_>>()
			.join("\n")
	}
}

async fn materialize_elf_jobs(args: &ElfArgs, jobs: &[LoadedJob]) -> Result<Vec<MaterializedJob>> {
	let base_dsn = env::var("ELF_PG_DSN")
		.map_err(|_| eyre::eyre!("ELF_PG_DSN must be set for ELF live real-world adapter."))?;
	let qdrant_url = env::var("ELF_QDRANT_GRPC_URL")
		.or_else(|_| env::var("ELF_QDRANT_URL"))
		.map_err(|_| eyre::eyre!("ELF_QDRANT_GRPC_URL or ELF_QDRANT_URL must be set."))?;

	fs::create_dir_all(&args.work_dir)?;

	let run_suffix = crate::short_hash(args.adapter_id.as_str());
	let runtime = BaselineRuntime {
		config_path: args.config.clone(),
		dsn: base_dsn,
		qdrant_url,
		collection: format!("elf_live_real_world_{run_suffix}"),
		docs_collection: format!("elf_live_real_world_docs_{run_suffix}"),
	};
	let service = crate::build_service(&runtime).await?;
	let mut out = Vec::with_capacity(jobs.len());

	for loaded in jobs {
		out.push(materialize_elf_job(&runtime, &service, loaded, args).await?);
	}

	drop(service);

	Ok(out)
}

async fn prepare_elf_ingest(
	runtime: &BaselineRuntime,
	service: &ElfService,
	loaded: &LoadedJob,
	args: &ElfArgs,
	corpus: &[CorpusText],
	project_id: &str,
) -> Result<IngestedCorpus> {
	let state_path = args.work_dir.join(format!("{}.json", crate::slug(&loaded.job.job_id)));

	if args.reuse_index {
		let raw = fs::read(&state_path).map_err(|err| {
			eyre::eyre!("Warm ELF ingest receipt is missing at {}: {err}", state_path.display())
		})?;
		let ingested = serde_json::from_slice(&raw)?;

		apply_native_operations(runtime, service, loaded, &ingested, project_id).await?;

		Ok(ingested)
	} else {
		if state_path.exists() {
			return Err(eyre::eyre!("Cold ELF state already exists at {}.", state_path.display()));
		}

		let ingested =
			crate::ingest_elf_corpus(service, loaded, &args.adapter_id, project_id, corpus).await?;

		crate::run_worker(runtime).await?;
		fs::write(&state_path, serde_json::to_vec_pretty(&ingested)?)?;

		Ok(ingested)
	}
}

async fn materialize_elf_job(
	runtime: &BaselineRuntime,
	service: &ElfService,
	loaded: &LoadedJob,
	args: &ElfArgs,
) -> Result<MaterializedJob> {
	if let Some(job) = crate::declared_encoding_job(&args.adapter_id, loaded) {
		return Ok(job);
	}
	if let Some(job) = crate::not_encoded_job(&args.adapter_id, loaded) {
		return Ok(job);
	}

	let corpus = crate::corpus_texts(loaded)?;
	let stored_corpus = crate::elf_stored_corpus_texts(&corpus)?;
	let project_id = crate::project_id_for_job(&loaded.job.job_id);
	let ingested =
		prepare_elf_ingest(runtime, service, loaded, args, &corpus, project_id.as_str()).await?;
	let (response, latency_ms) = search::search_elf_job(service, loaded, &project_id).await?;
	let evidence_ids = crate::search_response_evidence_ids(&response);
	let contexts = crate::search_response_contexts(&response);
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
		&args.adapter_id,
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
		&args.adapter_id,
		MaterializedJobInput {
			content: elf_job_content(loaded, &contexts, suite_selection.selected.content),
			evidence_ids: suite_selection.selected.evidence_ids,
			contexts: Some(contexts),
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

async fn apply_native_operations(
	runtime: &BaselineRuntime,
	service: &ElfService,
	loaded: &LoadedJob,
	ingested: &IngestedCorpus,
	project_id: &str,
) -> Result<()> {
	if loaded.job.operations.is_empty() {
		return Ok(());
	}

	let mut affected_note_ids = Vec::new();

	for operation in &loaded.job.operations {
		let note_ids =
			ingested.note_ids_by_evidence.get(&operation.evidence_id).ok_or_else(|| {
				eyre::eyre!(
					"ELF operation references unknown evidence {} for {}.",
					operation.evidence_id,
					loaded.job.job_id
				)
			})?;

		for note_id in note_ids {
			match operation.operation_type.as_str() {
				"update" => {
					let text = operation.text.clone().ok_or_else(|| {
						eyre::eyre!(
							"ELF update has no replacement text for {}.",
							operation.evidence_id
						)
					})?;
					let response = service
						.update(UpdateRequest {
							tenant_id: TENANT_ID.to_string(),
							project_id: project_id.to_string(),
							agent_id: AGENT_ID.to_string(),
							note_id: *note_id,
							text: Some(text),
							importance: None,
							confidence: None,
							ttl_days: None,
						})
						.await?;

					if response.op != NoteOp::Update {
						return Err(eyre::eyre!(
							"ELF update returned {:?} for {}.",
							response.op,
							operation.evidence_id
						));
					}
				},
				"delete" => {
					let response = service
						.delete(DeleteRequest {
							tenant_id: TENANT_ID.to_string(),
							project_id: project_id.to_string(),
							agent_id: AGENT_ID.to_string(),
							note_id: *note_id,
						})
						.await?;

					if response.op != NoteOp::Delete {
						return Err(eyre::eyre!(
							"ELF delete returned {:?} for {}.",
							response.op,
							operation.evidence_id
						));
					}
				},
				other => return Err(eyre::eyre!("Unsupported ELF operation {other}.")),
			}

			affected_note_ids.push(*note_id);
		}
	}

	if affected_note_ids.is_empty() {
		return Err(eyre::eyre!("ELF operation set affected no notes."));
	}

	crate::run_worker(runtime).await?;

	Ok(())
}
