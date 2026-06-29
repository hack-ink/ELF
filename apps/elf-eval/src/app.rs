use color_eyre::{Result, eyre};
use tracing_subscriber::EnvFilter;

mod cli;
mod compare;
mod dataset;
mod eval;
mod metrics;
mod trace_compare;
mod types;

pub use cli::{Args, SearchMode};

use types::{CompareOutput, EvalOutput};

pub async fn run(args: Args) -> Result<()> {
	let config_a = elf_config::load(&args.config_a)?;
	let filter = EnvFilter::new(config_a.service.log_level.clone());

	tracing_subscriber::fmt().with_env_filter(filter).init();

	if !args.trace_id.is_empty() {
		let Some(config_b_path) = &args.config_b else {
			return Err(eyre::eyre!("Trace compare mode requires --config-b."));
		};
		let config_b = elf_config::load(config_b_path)?;
		let output = trace_compare::trace_compare(
			args.config_a.as_path(),
			config_a,
			config_b_path.as_path(),
			config_b,
			&args,
		)
		.await?;
		let json = serde_json::to_string_pretty(&output)?;

		println!("{json}");

		return Ok(());
	}

	let dataset_path =
		args.dataset.as_ref().ok_or_else(|| eyre::eyre!("--dataset is required."))?;
	let dataset = dataset::load_dataset(dataset_path.as_path())?;
	let run_a =
		eval::eval_config(args.config_a.as_path(), config_a, &dataset, &args, args.search_mode)
			.await?;
	let search_mode_b = args.search_mode_b.unwrap_or(args.search_mode);

	if let Some(config_b_path) = &args.config_b {
		let config_b = elf_config::load(config_b_path)?;
		let run_b =
			eval::eval_config(config_b_path.as_path(), config_b, &dataset, &args, search_mode_b)
				.await?;
		let k = run_a.settings.top_k.min(run_b.settings.top_k).max(1);
		let (queries, policy_stability) =
			compare::build_compare_queries(&run_a.queries, &run_b.queries, k);
		let summary_delta = compare::diff_summary(&run_a.summary, &run_b.summary);
		let output = CompareOutput {
			dataset: run_a.dataset,
			settings_a: run_a.settings,
			settings_b: run_b.settings,
			summary_a: run_a.summary,
			summary_b: run_b.summary,
			summary_delta,
			policy_stability,
			queries,
		};
		let json = serde_json::to_string_pretty(&output)?;

		println!("{json}");

		return Ok(());
	}

	let output = EvalOutput {
		dataset: run_a.dataset,
		settings: run_a.settings,
		summary: run_a.summary,
		queries: run_a.queries,
	};
	let json = serde_json::to_string_pretty(&output)?;

	println!("{json}");

	Ok(())
}

#[cfg(test)] mod tests;
