use color_eyre::Result;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::{
	RUN_SCHEMA,
	cli::RunArgs,
	decision::{decide_project, summarize_decisions},
	github::{github_client, observe_project},
	io::{read_cursor, write_json, write_text},
	render::render_summary,
	types::RadarRun,
	validation::validate_cursor,
};

pub(super) async fn run_radar(args: RunArgs) -> Result<()> {
	let now = OffsetDateTime::now_utc();
	let generated_at = format_rfc3339(now)?;
	let run_id =
		args.run_id.unwrap_or_else(|| format!("external-memory-pattern-radar-{}", now.date()));
	let client = github_client(&args.github_token_env)?;
	let mut cursor = read_cursor(&args.cursor)?;
	let mut decisions = Vec::with_capacity(cursor.projects.len());

	for project in &mut cursor.projects {
		let prior = project.last_seen.clone();
		let observed = observe_project(project, args.mode, client.as_ref(), &generated_at).await?;

		decisions.push(decide_project(project, prior.as_ref(), &observed, args.mode));

		project.last_seen = Some(observed);
	}

	let summary = summarize_decisions(&decisions);

	cursor.generated_at = generated_at.clone();
	cursor.last_run = Some(RadarRun {
		schema: RUN_SCHEMA.to_string(),
		run_id,
		generated_at,
		mode: args.mode,
		summary,
		decisions,
	});

	validate_cursor(&cursor)?;

	let out_cursor = args.out_cursor.unwrap_or(args.cursor);

	write_json(&out_cursor, &cursor)?;
	write_text(&args.summary, &render_summary(&cursor)?)?;

	Ok(())
}

fn format_rfc3339(value: OffsetDateTime) -> Result<String> {
	Ok(value.format(&Rfc3339)?)
}
