#![allow(unused_crate_dependencies)]

//! CLI for evaluating trace-regression gates against stored traces.

#[path = "trace_regression_gate/cli.rs"] mod cli;
#[path = "trace_regression_gate/eval.rs"] mod eval;
#[path = "trace_regression_gate/gate.rs"] mod gate;
#[path = "trace_regression_gate/replay.rs"] mod replay;
#[path = "trace_regression_gate/reports.rs"] mod reports;
#[path = "trace_regression_gate/rows.rs"] mod rows;
#[path = "trace_regression_gate/storage.rs"] mod storage;

use std::fs;

use clap::Parser;
use color_eyre::{Result, eyre};
use tracing_subscriber::EnvFilter;

use self::{
	cli::Args,
	reports::{GateReport, GateSummary},
};
use elf_storage::db::Db;

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;

	let args = Args::parse();
	let cfg = elf_config::load(&args.config)?;
	let filter = EnvFilter::new(cfg.service.log_level.clone());

	tracing_subscriber::fmt().with_env_filter(filter).init();

	let gate = self::gate::load_gate_file(&args.gate)?;

	if gate.traces.is_empty() {
		return Err(eyre::eyre!("Gate JSON must include at least one trace."));
	}

	let gate_top_k = gate.top_k;
	let gate_retrieval_retention_rank = gate.retrieval_retention_rank;
	let db = Db::connect(&cfg.storage.postgres).await?;

	db.ensure_schema(cfg.storage.qdrant.vector_dim).await?;

	let mut traces = Vec::with_capacity(gate.traces.len());
	let mut breached_count = 0_usize;

	for trace in gate.traces {
		let thresholds = self::gate::merge_thresholds(gate.defaults, trace.thresholds);
		let report = self::eval::eval_trace(
			&db,
			&cfg,
			&args,
			gate_top_k,
			gate_retrieval_retention_rank,
			&trace,
			thresholds,
		)
		.await?;

		if !report.ok {
			breached_count += 1;
		}

		traces.push(report);
	}

	let summary =
		GateSummary { trace_count: traces.len(), breached_count, ok: breached_count == 0 };
	let report = GateReport {
		config_path: args.config.display().to_string(),
		gate_path: args.gate.display().to_string(),
		summary,
		traces,
	};
	let json = serde_json::to_string_pretty(&report)?;

	if let Some(out_path) = &args.out {
		fs::write(out_path, &json)?;
	} else {
		println!("{json}");
	}

	if !report.summary.ok {
		return Err(eyre::eyre!(
			"Trace regression gate breached: {}/{} traces failed thresholds.",
			report.summary.breached_count,
			report.summary.trace_count
		));
	}

	Ok(())
}
