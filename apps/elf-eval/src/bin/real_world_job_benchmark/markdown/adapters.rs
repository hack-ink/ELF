mod cells;
mod counts;
mod details;

use crate::markdown::{self, DEFAULT_ADAPTER_BEHAVIOR, RealWorldReport};

pub(super) fn render_markdown_capture_integration(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Capture And Integration Coverage\n\n");

	if report.adapter.behavior == DEFAULT_ADAPTER_BEHAVIOR {
		out.push_str("The real-world job runner is fixture-backed. This section separates encoded evidence from live adapter claims.\n\n");
	} else {
		out.push_str("This report scores materialized adapter responses. Capture and integration classes still describe the job corpus, not broad external adapter coverage.\n\n");
	}

	out.push_str("| Class | Behaviors |\n");
	out.push_str("| --- | --- |\n");
	out.push_str(&format!(
		"| real | {} |\n",
		markdown::md_list(report.capture_integration.real.as_slice())
	));
	out.push_str(&format!(
		"| fixture-backed | {} |\n",
		markdown::md_list(report.capture_integration.fixture_backed.as_slice())
	));
	out.push_str(&format!(
		"| mocked | {} |\n",
		markdown::md_list(report.capture_integration.mocked.as_slice())
	));
	out.push_str(&format!(
		"| blocked | {} |\n",
		markdown::md_list(report.capture_integration.blocked.as_slice())
	));
	out.push_str(&format!(
		"| not encoded | {} |\n",
		markdown::md_list(report.capture_integration.not_encoded.as_slice())
	));

	if !report.capture_integration.notes.is_empty() {
		out.push_str("\nNotes:\n");

		for note in &report.capture_integration.notes {
			out.push_str(&format!("- {}\n", markdown::md_cell(note.as_str())));
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
		markdown::md_inline(report.external_adapters.manifest_id.as_str())
	));
	out.push_str(&format!(
		"- Docker default: `{}` via `{}`; artifact dir `{}`\n",
		report.external_adapters.docker_isolation.default,
		markdown::md_inline(report.external_adapters.docker_isolation.compose_file.as_str()),
		markdown::md_inline(report.external_adapters.docker_isolation.artifact_dir.as_str())
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
		counts::adapter_status_counts_display(&summary.overall_status_counts)
	));
	out.push_str(&format!(
		"- Capability coverage statuses: `{}`\n",
		counts::adapter_status_counts_display(&summary.capability_status_counts)
	));
	out.push_str(&format!(
		"- Real-world suite statuses: `{}`\n",
		counts::adapter_status_counts_display(&summary.suite_status_counts)
	));

	if details::has_adapter_scenarios(report.external_adapters.adapters.as_slice()) {
		out.push_str(&format!(
			"- Scenario coverage statuses: `{}`\n",
			counts::adapter_status_counts_display(&summary.scenario_status_counts)
		));
		out.push_str(&format!(
			"- ELF scenario positions: `{}`\n",
			counts::scenario_position_counts_display(&summary.scenario_position_counts)
		));
		out.push_str(&format!(
			"- Scenario comparison outcomes: `{}`\n",
			counts::scenario_outcome_counts_display(&summary.scenario_outcome_counts)
		));
	}

	out.push('\n');
	out.push_str("| Project | Adapter | Evidence Class | Overall | Setup | Run | Result | Docker | Suites | Evidence |\n");
	out.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");

	for adapter in &report.external_adapters.adapters {
		out.push_str(&format!(
			"| {} | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | {} | {} |\n",
			markdown::md_cell(adapter.project.as_str()),
			markdown::md_inline(adapter.adapter_id.as_str()),
			markdown::md_inline(adapter.evidence_class.as_str()),
			markdown::adapter_status_str(adapter.overall_status),
			markdown::adapter_status_str(adapter.setup.status),
			markdown::adapter_status_str(adapter.run.status),
			markdown::adapter_status_str(adapter.result.status),
			adapter.docker_default,
			cells::adapter_suite_cell(adapter.suites.as_slice()),
			cells::adapter_evidence_cell(adapter)
		));
	}

	out.push_str("\n### Adapter Capability Details\n\n");
	out.push_str("| Adapter | Capability | Status | Evidence |\n");
	out.push_str("| --- | --- | --- | --- |\n");

	for adapter in &report.external_adapters.adapters {
		for capability in &adapter.capabilities {
			out.push_str(&format!(
				"| `{}` | {} | `{}` | {} |\n",
				markdown::md_inline(adapter.adapter_id.as_str()),
				markdown::md_cell(capability.capability.as_str()),
				markdown::adapter_status_str(capability.status),
				markdown::md_cell(capability.evidence.as_str())
			));
		}
	}

	details::render_markdown_adapter_scenarios(out, report.external_adapters.adapters.as_slice());
	details::render_markdown_adapter_execution_metadata(
		out,
		report.external_adapters.adapters.as_slice(),
	);

	out.push('\n');
}
