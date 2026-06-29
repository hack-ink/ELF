use super::*;

pub(super) fn render_markdown_capture_integration(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Capture And Integration Coverage\n\n");

	if report.adapter.behavior == DEFAULT_ADAPTER_BEHAVIOR {
		out.push_str("The real-world job runner is fixture-backed. This section separates encoded evidence from live adapter claims.\n\n");
	} else {
		out.push_str("This report scores materialized adapter responses. Capture and integration classes still describe the job corpus, not broad external adapter coverage.\n\n");
	}

	out.push_str("| Class | Behaviors |\n");
	out.push_str("| --- | --- |\n");
	out.push_str(&format!("| real | {} |\n", md_list(report.capture_integration.real.as_slice())));
	out.push_str(&format!(
		"| fixture-backed | {} |\n",
		md_list(report.capture_integration.fixture_backed.as_slice())
	));
	out.push_str(&format!(
		"| mocked | {} |\n",
		md_list(report.capture_integration.mocked.as_slice())
	));
	out.push_str(&format!(
		"| blocked | {} |\n",
		md_list(report.capture_integration.blocked.as_slice())
	));
	out.push_str(&format!(
		"| not encoded | {} |\n",
		md_list(report.capture_integration.not_encoded.as_slice())
	));

	if !report.capture_integration.notes.is_empty() {
		out.push_str("\nNotes:\n");

		for note in &report.capture_integration.notes {
			out.push_str(&format!("- {}\n", md_cell(note.as_str())));
		}
	}

	out.push('\n');
}

pub(super) fn render_markdown_external_adapters(out: &mut String, report: &RealWorldReport) {
	out.push_str("## External Adapter Coverage\n\n");

	if report.external_adapters.adapters.is_empty() {
		out.push_str("No external adapter coverage manifest was loaded for this report.\n\n");

		return;
	}

	let summary = &report.external_adapters.summary;

	out.push_str("This section is manifest-backed. It records external adapter coverage and blockers, but it does not convert live-baseline retrieval results into real-world suite wins.\n\n");
	out.push_str(&format!(
		"- Manifest: `{}`\n",
		md_inline(report.external_adapters.manifest_id.as_str())
	));
	out.push_str(&format!(
		"- Docker default: `{}` via `{}`; artifact dir `{}`\n",
		report.external_adapters.docker_isolation.default,
		md_inline(report.external_adapters.docker_isolation.compose_file.as_str()),
		md_inline(report.external_adapters.docker_isolation.artifact_dir.as_str())
	));
	out.push_str(&format!(
		"- Adapter records: `{}` total, `{}` external project(s), `{}` Docker-default, `{}` requiring host-global installs\n",
		summary.adapter_count,
		summary.external_project_count,
		summary.docker_default_count,
		summary.host_global_install_required_count
	));
	out.push_str(&format!(
		"- Evidence classes: `{}` fixture-backed, `{}` live-baseline-only, `{}` live real-world, `{}` research-gate\n",
		summary.fixture_backed_count,
		summary.live_baseline_only_count,
		summary.live_real_world_count,
		summary.research_gate_count
	));
	out.push_str(&format!(
		"- Overall statuses: `{}`\n",
		adapter_status_counts_display(&summary.overall_status_counts)
	));
	out.push_str(&format!(
		"- Capability coverage statuses: `{}`\n",
		adapter_status_counts_display(&summary.capability_status_counts)
	));
	out.push_str(&format!(
		"- Real-world suite statuses: `{}`\n",
		adapter_status_counts_display(&summary.suite_status_counts)
	));

	if has_adapter_scenarios(report.external_adapters.adapters.as_slice()) {
		out.push_str(&format!(
			"- Scenario coverage statuses: `{}`\n",
			adapter_status_counts_display(&summary.scenario_status_counts)
		));
		out.push_str(&format!(
			"- ELF scenario positions: `{}`\n",
			scenario_position_counts_display(&summary.scenario_position_counts)
		));
		out.push_str(&format!(
			"- Scenario comparison outcomes: `{}`\n",
			scenario_outcome_counts_display(&summary.scenario_outcome_counts)
		));
	}

	out.push('\n');
	out.push_str("| Project | Adapter | Evidence Class | Overall | Setup | Run | Result | Docker | Suites | Evidence |\n");
	out.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");

	for adapter in &report.external_adapters.adapters {
		out.push_str(&format!(
			"| {} | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | {} | {} |\n",
			md_cell(adapter.project.as_str()),
			md_inline(adapter.adapter_id.as_str()),
			md_inline(adapter.evidence_class.as_str()),
			adapter_status_str(adapter.overall_status),
			adapter_status_str(adapter.setup.status),
			adapter_status_str(adapter.run.status),
			adapter_status_str(adapter.result.status),
			adapter.docker_default,
			adapter_suite_cell(adapter.suites.as_slice()),
			adapter_evidence_cell(adapter)
		));
	}

	out.push_str("\n### Adapter Capability Details\n\n");
	out.push_str("| Adapter | Capability | Status | Evidence |\n");
	out.push_str("| --- | --- | --- | --- |\n");

	for adapter in &report.external_adapters.adapters {
		for capability in &adapter.capabilities {
			out.push_str(&format!(
				"| `{}` | {} | `{}` | {} |\n",
				md_inline(adapter.adapter_id.as_str()),
				md_cell(capability.capability.as_str()),
				adapter_status_str(capability.status),
				md_cell(capability.evidence.as_str())
			));
		}
	}

	render_markdown_adapter_scenarios(out, report.external_adapters.adapters.as_slice());
	render_markdown_adapter_execution_metadata(out, report.external_adapters.adapters.as_slice());

	out.push('\n');
}

fn render_markdown_adapter_scenarios(out: &mut String, adapters: &[ExternalAdapterReport]) {
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
				md_inline(adapter.adapter_id.as_str()),
				md_inline(scenario.scenario_id.as_str()),
				scenario
					.suite_id
					.as_deref()
					.map(|suite| format!("`{}`", md_inline(suite)))
					.unwrap_or_else(|| "`none`".to_string()),
				adapter_status_str(scenario.status),
				scenario_comparison_outcome_str(scenario_comparison_outcome(scenario)),
				adapter_scenario_evidence_cell(scenario)
			));
		}
	}
}

fn render_markdown_adapter_execution_metadata(
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
			md_inline(adapter.adapter_id.as_str()),
			adapter_sources_cell(metadata.sources.as_slice()),
			md_cell(metadata.setup_path.as_str()),
			md_cell(metadata.runtime_boundary.as_str()),
			md_cell(metadata.resource_expectation.as_str()),
			md_list(metadata.retry_guidance.as_slice()),
			md_cell(metadata.research_depth.as_deref().unwrap_or("not recorded"))
		));
	}
}

fn has_adapter_scenarios(adapters: &[ExternalAdapterReport]) -> bool {
	adapters.iter().any(|adapter| !adapter.scenarios.is_empty())
}

fn adapter_status_counts_display(counts: &AdapterStatusCounts) -> String {
	[
		("real", counts.real),
		("mocked", counts.mocked),
		("unsupported", counts.unsupported),
		("blocked", counts.blocked),
		("incomplete", counts.incomplete),
		("wrong_result", counts.wrong_result),
		("lifecycle_fail", counts.lifecycle_fail),
		("pass", counts.pass),
		("not_encoded", counts.not_encoded),
	]
	.into_iter()
	.filter(|(_, count)| *count > 0)
	.map(|(status, count)| format!("{status}={count}"))
	.collect::<Vec<_>>()
	.join(", ")
}

fn scenario_position_counts_display(counts: &ScenarioPositionCounts) -> String {
	[
		("wins", counts.wins),
		("ties", counts.ties),
		("loses", counts.loses),
		("untested", counts.untested),
	]
	.into_iter()
	.filter(|(_, count)| *count > 0)
	.map(|(position, count)| format!("{position}={count}"))
	.collect::<Vec<_>>()
	.join(", ")
}

fn scenario_outcome_counts_display(counts: &ScenarioOutcomeCounts) -> String {
	[
		("win", counts.win),
		("tie", counts.tie),
		("loss", counts.loss),
		("not_tested", counts.not_tested),
		("blocked", counts.blocked),
		("non_goal", counts.non_goal),
	]
	.into_iter()
	.filter(|(_, count)| *count > 0)
	.map(|(outcome, count)| format!("{outcome}={count}"))
	.collect::<Vec<_>>()
	.join(", ")
}

fn adapter_suite_cell(suites: &[AdapterSuiteCoverage]) -> String {
	if suites.is_empty() {
		return "`none`".to_string();
	}

	suites
		.iter()
		.map(|suite| {
			format!(
				"`{}`: `{}`",
				md_inline(suite.suite_id.as_str()),
				adapter_status_str(suite.status)
			)
		})
		.collect::<Vec<_>>()
		.join("<br>")
}

fn adapter_evidence_cell(adapter: &ExternalAdapterReport) -> String {
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

	format!("setup: `{}`<br>result: `{}`", md_inline(setup), md_inline(result))
}

fn adapter_scenario_evidence_cell(scenario: &AdapterScenarioJudgment) -> String {
	let evidence = md_cell(scenario.evidence.as_str());
	let command = scenario
		.command
		.as_deref()
		.map(|command| format!("<br>command: `{}`", md_inline(command)))
		.unwrap_or_default();
	let artifact = scenario
		.artifact
		.as_deref()
		.map(|artifact| format!("<br>artifact: `{}`", md_inline(artifact)))
		.unwrap_or_default();

	format!("{evidence}{command}{artifact}")
}

fn adapter_sources_cell(sources: &[AdapterSource]) -> String {
	if sources.is_empty() {
		return "`none`".to_string();
	}

	sources
		.iter()
		.map(|source| {
			format!(
				"[{}]({}): {}",
				md_cell(source.label.as_str()),
				md_url(source.url.as_str()),
				md_cell(source.evidence.as_str())
			)
		})
		.collect::<Vec<_>>()
		.join("<br>")
}
