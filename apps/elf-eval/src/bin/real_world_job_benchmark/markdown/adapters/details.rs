use crate::markdown::{self, ExternalAdapterReport, adapters::cells};

pub(in crate::markdown::adapters) fn has_adapter_scenarios(
	adapters: &[ExternalAdapterReport],
) -> bool {
	adapters.iter().any(|adapter| !adapter.scenarios.is_empty())
}

pub(in crate::markdown::adapters) fn render_markdown_adapter_scenarios(
	out: &mut String,
	adapters: &[ExternalAdapterReport],
) {
	if !has_adapter_scenarios(adapters) {
		return;
	}

	out.push_str("\n### Adapter Scenario Judgments\n\n");
	out.push_str("| Adapter | Scenario | Suite | Status | Outcome | Evidence |\n");
	out.push_str("| --- | --- | --- | --- | --- | --- |\n");

	for adapter in adapters {
		for scenario in &adapter.scenarios {
			out.push_str(&format!(
				"| `{}` | `{}` | {} | `{}` | `{}` | {} |\n",
				markdown::md_inline(adapter.adapter_id.as_str()),
				markdown::md_inline(scenario.scenario_id.as_str()),
				scenario
					.suite_id
					.as_deref()
					.map(|suite| format!("`{}`", markdown::md_inline(suite)))
					.unwrap_or_else(|| "`none`".to_string()),
				markdown::adapter_status_str(scenario.status),
				markdown::scenario_comparison_outcome_str(markdown::scenario_comparison_outcome(
					scenario
				)),
				cells::adapter_scenario_evidence_cell(scenario)
			));
		}
	}
}

pub(in crate::markdown::adapters) fn render_markdown_adapter_execution_metadata(
	out: &mut String,
	adapters: &[ExternalAdapterReport],
) {
	let mut wrote_header = false;

	for adapter in adapters {
		let Some(metadata) = &adapter.execution_metadata else {
			continue;
		};

		if !wrote_header {
			out.push_str("\n### Adapter Execution Metadata\n\n");
			out.push_str("| Adapter | Sources | Setup Path | Runtime Boundary | Resource Expectation | Retry Guidance | Research Depth |\n");
			out.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");

			wrote_header = true;
		}

		out.push_str(&format!(
			"| `{}` | {} | {} | {} | {} | {} | {} |\n",
			markdown::md_inline(adapter.adapter_id.as_str()),
			cells::adapter_sources_cell(metadata.sources.as_slice()),
			markdown::md_cell(metadata.setup_path.as_str()),
			markdown::md_cell(metadata.runtime_boundary.as_str()),
			markdown::md_cell(metadata.resource_expectation.as_str()),
			markdown::md_list(metadata.retry_guidance.as_slice()),
			markdown::md_cell(metadata.research_depth.as_deref().unwrap_or("not recorded"))
		));
	}
}
