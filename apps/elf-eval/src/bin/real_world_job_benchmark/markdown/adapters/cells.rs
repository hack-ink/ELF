use crate::markdown::{
	self, AdapterScenarioJudgment, AdapterSource, AdapterSuiteCoverage, ExternalAdapterReport,
};

pub(in crate::markdown::adapters) fn adapter_suite_cell(suites: &[AdapterSuiteCoverage]) -> String {
	if suites.is_empty() {
		return "`none`".to_string();
	}

	suites
		.iter()
		.map(|suite| {
			format!(
				"`{}`: `{}`",
				markdown::md_inline(suite.suite_id.as_str()),
				markdown::adapter_status_str(suite.status)
			)
		})
		.collect::<Vec<_>>()
		.join("<br>")
}

pub(in crate::markdown::adapters) fn adapter_evidence_cell(
	adapter: &ExternalAdapterReport,
) -> String {
	let setup = adapter
		.setup
		.command
		.as_deref()
		.or(adapter.setup.artifact.as_deref())
		.unwrap_or(adapter.setup.evidence.as_str());
	let result = adapter
		.result
		.artifact
		.as_deref()
		.or(adapter.result.command.as_deref())
		.unwrap_or(adapter.result.evidence.as_str());

	format!("setup: `{}`<br>result: `{}`", markdown::md_inline(setup), markdown::md_inline(result))
}

pub(in crate::markdown::adapters) fn adapter_scenario_evidence_cell(
	scenario: &AdapterScenarioJudgment,
) -> String {
	let evidence = markdown::md_cell(scenario.evidence.as_str());
	let command = scenario
		.command
		.as_deref()
		.map(|command| format!("<br>command: `{}`", markdown::md_inline(command)))
		.unwrap_or_default();
	let artifact = scenario
		.artifact
		.as_deref()
		.map(|artifact| format!("<br>artifact: `{}`", markdown::md_inline(artifact)))
		.unwrap_or_default();

	format!("{evidence}{command}{artifact}")
}

pub(in crate::markdown::adapters) fn adapter_sources_cell(sources: &[AdapterSource]) -> String {
	if sources.is_empty() {
		return "`none`".to_string();
	}

	sources
		.iter()
		.map(|source| {
			format!(
				"[{}]({}): {}",
				markdown::md_cell(source.label.as_str()),
				markdown::md_url(source.url.as_str()),
				markdown::md_cell(source.evidence.as_str())
			)
		})
		.collect::<Vec<_>>()
		.join("<br>")
}
