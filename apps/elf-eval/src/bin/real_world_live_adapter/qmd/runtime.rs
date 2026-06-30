use crate::{
	AdapterKind, CommandEvidence, LoadedJob, MaterializedJob, MaterializedOutput, QmdArgs, Result,
	aggregate_status, fs,
	qmd::{checkout, job},
};

pub(crate) fn run_qmd(args: QmdArgs) -> Result<()> {
	let jobs = crate::load_jobs(&args.fixtures)?;
	let result = materialize_qmd_jobs(&args, &jobs);
	let materialized = match result {
		Ok(jobs) => jobs,
		Err(err) =>
			crate::failure_jobs(&args.adapter_id, &jobs, "qmd_cli_runtime", err.to_string()),
	};

	crate::write_materialized_output(MaterializedOutput {
		adapter_id: &args.adapter_id,
		adapter_kind: AdapterKind::QmdCliRuntime,
		fixtures: &args.fixtures,
		out_fixtures: &args.out_fixtures,
		evidence_out: &args.evidence_out,
		jobs: &jobs,
		materialized: &materialized,
		command_evidence: vec![CommandEvidence {
			label: "qmd_cli_runtime".to_string(),
			status: aggregate_status(&materialized),
			command: "cargo run -p elf-eval --bin real_world_live_adapter -- qmd".to_string(),
			artifact: Some(args.evidence_out.display().to_string()),
			reason: "qmd live adapter used collection add, update, embed, and query --json."
				.to_string(),
		}],
		metadata: None,
	})
}

fn materialize_qmd_jobs(args: &QmdArgs, jobs: &[LoadedJob]) -> Result<Vec<MaterializedJob>> {
	fs::create_dir_all(&args.work_dir)?;

	let log_path = args.work_dir.join("qmd-live-real-world.log");

	checkout::ensure_qmd_checkout(args, &log_path)?;

	let mut out = Vec::with_capacity(jobs.len());

	for loaded in jobs {
		out.push(job::materialize_qmd_job(args, loaded, &log_path)?);
	}

	Ok(out)
}
