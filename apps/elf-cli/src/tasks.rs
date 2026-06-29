use std::{
	collections::BTreeMap,
	io::{self, Write as _},
	path::Path,
	process::Command,
};

use color_eyre::{Result, eyre};

use crate::{
	args::{BackfillArgs, BenchmarkArgs, BenchmarkCommand, BenchmarkReportArgs, BenchmarkRunArgs},
	json::write_json,
};

pub(crate) fn run_backfill(args: BackfillArgs) -> Result<()> {
	let task = if args.hundred_k {
		"baseline-backfill-100k-docker"
	} else if args.ten_k {
		"baseline-backfill-10k-docker"
	} else {
		"baseline-backfill-docker"
	};
	let mut env = BTreeMap::new();

	if let Some(docs) = args.docs {
		env.insert("ELF_BASELINE_BACKFILL_DOCS".to_string(), docs.to_string());
	}
	if let Some(worker_concurrency) = args.worker_concurrency {
		env.insert("ELF_BASELINE_WORKER_CONCURRENCY".to_string(), worker_concurrency.to_string());
	}

	if args.enable_expensive {
		env.insert("ELF_BASELINE_ENABLE_EXPENSIVE".to_string(), "1".to_string());
	}

	run_cargo_make("elf.cli.backfill/v1", task, env, args.dry_run, args.output.pretty)
}

pub(crate) fn run_benchmark(args: BenchmarkArgs) -> Result<()> {
	match args.command {
		BenchmarkCommand::Run(args) => run_benchmark_run(args),
		BenchmarkCommand::Report(args) => run_benchmark_report(args),
	}
}

fn run_benchmark_run(args: BenchmarkRunArgs) -> Result<()> {
	let task = args.kind.task_name();
	let mut env = BTreeMap::new();

	if let Some(projects) = args.projects {
		env.insert("ELF_BASELINE_PROJECTS".to_string(), projects);
	}
	if let Some(profile) = args.profile {
		env.insert("ELF_BASELINE_PROFILE".to_string(), profile);
	}
	if let Some(path) = args.production_corpus_manifest {
		env.insert("ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST".to_string(), path_display(&path));
	}
	if let Some(path) = args.private_addendum {
		env.insert("ELF_BASELINE_PRIVATE_ADDENDUM".to_string(), path_display(&path));
	}
	if let Some(seconds) = args.soak_seconds {
		env.insert("ELF_BASELINE_SOAK_SECONDS".to_string(), seconds.to_string());
	}

	run_cargo_make("elf.cli.benchmark_run/v1", task, env, args.dry_run, args.output.pretty)
}

fn run_benchmark_report(args: BenchmarkReportArgs) -> Result<()> {
	let mut env = BTreeMap::new();

	if let Some(path) = args.report {
		env.insert("ELF_BASELINE_REPORT".to_string(), path_display(&path));
	}
	if let Some(path) = args.out {
		env.insert("ELF_BASELINE_MARKDOWN_REPORT".to_string(), path_display(&path));
	}

	run_cargo_make(
		"elf.cli.benchmark_report/v1",
		"baseline-live-report",
		env,
		args.dry_run,
		args.output.pretty,
	)
}

fn run_cargo_make(
	schema: &str,
	task: &str,
	env: BTreeMap<String, String>,
	dry_run: bool,
	pretty: bool,
) -> Result<()> {
	let command = serde_json::json!({
		"program": "cargo",
		"args": ["make", task],
		"env": env,
	});

	if dry_run {
		let output = serde_json::json!({
			"schema": schema,
			"dry_run": true,
			"command": command,
		});

		return write_json(&output, pretty);
	}

	let output = Command::new("cargo").arg("make").arg(task).envs(env.iter()).output()?;

	io::stderr().write_all(&output.stdout)?;
	io::stderr().write_all(&output.stderr)?;

	let status_code = output.status.code();
	let summary = serde_json::json!({
		"schema": schema,
		"dry_run": false,
		"command": command,
		"status_code": status_code,
		"success": output.status.success(),
	});

	write_json(&summary, pretty)?;

	if output.status.success() {
		Ok(())
	} else {
		Err(eyre::eyre!("cargo make {task} failed with status {status_code:?}."))
	}
}

fn path_display(path: &Path) -> String {
	path.display().to_string()
}
