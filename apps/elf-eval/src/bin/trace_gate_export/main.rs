#![allow(unused_crate_dependencies)]

//! CLI for exporting trace fixtures used by regression gates.

mod cli;
mod fetch;
mod render;
mod rows;
mod sql;

use std::fs;

use clap::Parser;
use color_eyre::Result;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use self::cli::Args;
use elf_storage::db::Db;

fn normalize_trace_ids(trace_ids: &[Uuid]) -> Vec<Uuid> {
	let mut out = trace_ids.to_vec();

	out.sort_unstable();
	out.dedup();

	out
}

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;

	let args = Args::parse();
	let cfg = elf_config::load(&args.config)?;
	let filter = EnvFilter::new(cfg.service.log_level.clone());

	tracing_subscriber::fmt().with_env_filter(filter).init();

	let trace_ids = normalize_trace_ids(&args.trace_id);
	let db = Db::connect(&cfg.storage.postgres).await?;

	db.ensure_schema(cfg.storage.qdrant.vector_dim).await?;

	let traces = self::fetch::fetch_traces(&db, &trace_ids).await?;
	let candidates = self::fetch::fetch_candidates(&db, &trace_ids).await?;
	let items = if args.include_items {
		self::fetch::fetch_items(&db, &trace_ids).await?
	} else {
		Vec::new()
	};
	let (stages, stage_items) = if args.include_stages {
		let stages = self::fetch::fetch_stages(&db, &trace_ids).await?;
		let stage_ids: Vec<Uuid> = stages.iter().map(|row| row.stage_id).collect();
		let stage_items = self::fetch::fetch_stage_items(&db, &stage_ids).await?;

		(stages, stage_items)
	} else {
		(Vec::new(), Vec::new())
	};
	let sql = self::render::render_fixture_sql(
		&args,
		&traces,
		&candidates,
		&items,
		&stages,
		&stage_items,
	)?;

	if let Some(out_path) = &args.out {
		fs::write(out_path, sql)?;
	} else {
		print!("{sql}");
	}

	Ok(())
}
