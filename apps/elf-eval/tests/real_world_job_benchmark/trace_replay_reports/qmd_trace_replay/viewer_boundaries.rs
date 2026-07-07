use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(super) fn assert_trace_replay_viewer_blocker_boundaries(
	readme: &str,
	markdown: &str,
	adoption_report: &str,
	report: &Value,
	adoption_json: &Value,
) -> Result<()> {
	let checked_surfaces = [
		support::collapse_whitespace(readme),
		support::collapse_whitespace(markdown),
		support::collapse_whitespace(adoption_report),
		report.to_string(),
		adoption_json.to_string(),
	];

	for surface in checked_surfaces {
		assert!(!surface.contains("blocked or not encoded"));
	}

	assert!(
		support::collapse_whitespace(readme)
			.contains("claude-mem viewer flows remain blocked until Docker-contained")
	);
	assert!(
		support::collapse_whitespace(markdown)
			.contains("claude-mem UI repair paths remain blocked until Docker-contained")
	);
	assert!(
		support::collapse_whitespace(adoption_report)
			.contains("claude-mem viewer workflows remain blocked until Docker-contained")
	);

	Ok(())
}
