use crate::{
	EVIDENCE_SCHEMA, LoadedJob, MaterializationEvidence, MaterializationStatus, MaterializedJob,
	MaterializedJobEvidence, MaterializedJobInput, MaterializedOutput, Path, PathBuf, Result,
	Value, eyre, fs, serde_json,
};

pub(super) fn failure_jobs(
	adapter_id: &str,
	jobs: &[LoadedJob],
	stage: &str,
	reason: String,
) -> Vec<MaterializedJob> {
	jobs.iter()
		.map(|job| {
			crate::materialized_job(
				job,
				adapter_id,
				MaterializedJobInput {
					content: String::new(),
					evidence_ids: Vec::new(),
					pages: Vec::new(),
					latency_ms: 0.0,
					indexing_latency_ms: None,
					returned_count: 0,
					trace_id: None,
					failure: Some(format!("{stage}: {reason}")),
					source_mappings: Vec::new(),
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
			)
		})
		.collect()
}

pub(super) fn write_materialized_output(output: MaterializedOutput<'_>) -> Result<()> {
	if output.out_fixtures.exists() {
		fs::remove_dir_all(output.out_fixtures)?;
	}

	fs::create_dir_all(output.out_fixtures)?;

	for (loaded, materialized) in output.jobs.iter().zip(output.materialized) {
		let mut value = loaded.value.clone();
		let mut adapter_response =
			value["corpus"]["adapter_response"].as_object().cloned().unwrap_or_default();

		adapter_response.insert(
			"adapter_id".to_string(),
			serde_json::to_value(&materialized.response.adapter_id)?,
		);
		adapter_response
			.insert("answer".to_string(), serde_json::to_value(&materialized.response.answer)?);

		if let Some(consolidation) = &materialized.response.consolidation {
			adapter_response.insert("consolidation".to_string(), consolidation.clone());
		} else if loaded.job.suite == "consolidation" {
			adapter_response.remove("consolidation");
		}

		value["corpus"]["adapter_response"] = Value::Object(adapter_response);

		if let Some(operator_debug) = &materialized.operator_debug {
			value["operator_debug"] = operator_debug.clone();
		}
		if let Some(capture) = &materialized.evidence.capture {
			crate::apply_capture_runtime_source_refs(&mut value, capture);

			value["capture_materialization"] = serde_json::to_value(capture)?;
		}

		if matches!(
			materialized.evidence.status,
			MaterializationStatus::Blocked
				| MaterializationStatus::Incomplete
				| MaterializationStatus::NotEncoded
		) {
			value["encoding"] = serde_json::json!({
				"status": materialization_status_str(materialized.evidence.status),
				"reason": materialized.evidence.failure.clone().unwrap_or_else(|| {
					"Live adapter did not complete this job as a pass/fail check.".to_string()
				}),
			});
		}

		let output_path = output_fixture_path(output.fixtures, output.out_fixtures, &loaded.path)?;

		if let Some(parent) = output_path.parent() {
			fs::create_dir_all(parent)?;
		}

		fs::write(output_path, serde_json::to_string_pretty(&value)?)?;
	}

	let evidence = MaterializationEvidence {
		schema: EVIDENCE_SCHEMA,
		adapter_id: output.adapter_id.to_string(),
		adapter_kind: output.adapter_kind,
		status: aggregate_status(output.materialized),
		fixtures: output.fixtures.display().to_string(),
		generated_fixtures: output.out_fixtures.display().to_string(),
		command_evidence: output.command_evidence,
		jobs: output.materialized.iter().map(|job| clone_job_evidence(&job.evidence)).collect(),
		metadata: output.metadata,
	};

	if let Some(parent) = output.evidence_out.parent() {
		fs::create_dir_all(parent)?;
	}

	fs::write(output.evidence_out, serde_json::to_string_pretty(&evidence)?)?;

	Ok(())
}

pub(super) fn aggregate_status(jobs: &[MaterializedJob]) -> MaterializationStatus {
	if jobs.iter().any(|job| job.evidence.status == MaterializationStatus::Incomplete) {
		MaterializationStatus::Incomplete
	} else if jobs.iter().any(|job| job.evidence.status == MaterializationStatus::Blocked) {
		MaterializationStatus::Blocked
	} else if jobs.iter().any(|job| job.evidence.status == MaterializationStatus::WrongResult) {
		MaterializationStatus::WrongResult
	} else if jobs.iter().any(|job| job.evidence.status == MaterializationStatus::NotEncoded) {
		MaterializationStatus::NotEncoded
	} else {
		MaterializationStatus::Pass
	}
}

fn clone_job_evidence(evidence: &MaterializedJobEvidence) -> MaterializedJobEvidence {
	MaterializedJobEvidence {
		job_id: evidence.job_id.clone(),
		suite: evidence.suite.clone(),
		title: evidence.title.clone(),
		status: evidence.status,
		query: evidence.query.clone(),
		evidence_ids: evidence.evidence_ids.clone(),
		returned_count: evidence.returned_count,
		indexing_latency_ms: evidence.indexing_latency_ms,
		latency_ms: evidence.latency_ms,
		trace_id: evidence.trace_id,
		failure: evidence.failure.clone(),
		source_mappings: evidence.source_mappings.clone(),
		operator_debug: evidence.operator_debug.clone(),
		capture: evidence.capture.clone(),
		consolidation: evidence.consolidation.clone(),
		knowledge: evidence.knowledge.clone(),
		temporal_reconciliation: evidence.temporal_reconciliation.clone(),
		dreaming_readback: evidence.dreaming_readback.clone(),
	}
}

fn materialization_status_str(status: MaterializationStatus) -> &'static str {
	match status {
		MaterializationStatus::Pass => "pass",
		MaterializationStatus::WrongResult => "wrong_result",
		MaterializationStatus::Blocked => "blocked",
		MaterializationStatus::Incomplete => "incomplete",
		MaterializationStatus::NotEncoded => "not_encoded",
	}
}

fn output_fixture_path(fixtures: &Path, out_fixtures: &Path, fixture: &Path) -> Result<PathBuf> {
	if fixtures.is_dir() {
		let relative = fixture.strip_prefix(fixtures).map_err(|err| {
			eyre::eyre!(
				"Fixture path {} is not under fixture root {}: {err}",
				fixture.display(),
				fixtures.display()
			)
		})?;

		return Ok(out_fixtures.join(relative));
	}

	let file_name = fixture
		.file_name()
		.ok_or_else(|| eyre::eyre!("Fixture path {} has no file name.", fixture.display()))?;

	Ok(out_fixtures.join(file_name))
}
