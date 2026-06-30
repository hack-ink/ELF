use crate::scoreboard::{AdapterCoverageStatus, ExternalAdapterReport};

pub(super) fn external_project_same_corpus(adapters: &[&ExternalAdapterReport]) -> bool {
	let needles = &["same-corpus", "same corpus", "same_corpus", "shared corpus"];

	adapters.iter().any(|adapter| {
		text_mentions_any(adapter.adapter_kind.as_str(), needles)
			|| adapter_has_reported_same_corpus_text(adapter, needles)
	})
}

pub(super) fn external_project_source_id_mapped(adapters: &[&ExternalAdapterReport]) -> bool {
	let needles = &[
		"source-id mapped",
		"source ids mapped",
		"maps to source ids",
		"mapped to source ids",
		"maps back to source ids",
		"map to generated evidence ids",
		"mapped to generated evidence ids",
		"evidence ids match",
	];

	adapters.iter().any(|adapter| adapter_has_passing_text(adapter, needles))
}

pub(super) fn adapter_has_container_digest(adapter: &ExternalAdapterReport) -> bool {
	adapter.setup.evidence.contains("sha256:")
		|| adapter.run.evidence.contains("sha256:")
		|| adapter.result.evidence.contains("sha256:")
		|| adapter.evidence.iter().any(|evidence| {
			evidence.reference.contains("sha256:") || evidence.reference.contains("digest")
		})
}

fn adapter_has_passing_text(adapter: &ExternalAdapterReport, needles: &[&str]) -> bool {
	adapter_status_mentions_any(adapter.setup.status, adapter.setup.evidence.as_str(), needles)
		|| adapter_status_mentions_any(adapter.run.status, adapter.run.evidence.as_str(), needles)
		|| adapter_status_mentions_any(
			adapter.result.status,
			adapter.result.evidence.as_str(),
			needles,
		) || adapter.capabilities.iter().any(|capability| {
		adapter_status_mentions_any(capability.status, capability.capability.as_str(), needles)
			|| adapter_status_mentions_any(capability.status, capability.evidence.as_str(), needles)
	}) || adapter.suites.iter().any(|suite| {
		adapter_status_mentions_any(suite.status, suite.suite_id.as_str(), needles)
			|| adapter_status_mentions_any(suite.status, suite.evidence.as_str(), needles)
	}) || adapter.scenarios.iter().any(|scenario| {
		adapter_status_mentions_any(scenario.status, scenario.scenario_id.as_str(), needles)
			|| adapter_status_mentions_any(scenario.status, scenario.evidence.as_str(), needles)
	})
}

fn adapter_has_reported_same_corpus_text(
	adapter: &ExternalAdapterReport,
	needles: &[&str],
) -> bool {
	adapter_status_reports_same_corpus(
		adapter.setup.status,
		adapter.setup.evidence.as_str(),
		needles,
	) || adapter_status_reports_same_corpus(
		adapter.run.status,
		adapter.run.evidence.as_str(),
		needles,
	) || adapter_status_reports_same_corpus(
		adapter.result.status,
		adapter.result.evidence.as_str(),
		needles,
	) || adapter.capabilities.iter().any(|capability| {
		adapter_status_reports_same_corpus(
			capability.status,
			capability.capability.as_str(),
			needles,
		) || adapter_status_reports_same_corpus(
			capability.status,
			capability.evidence.as_str(),
			needles,
		)
	}) || adapter.suites.iter().any(|suite| {
		adapter_status_reports_same_corpus(suite.status, suite.suite_id.as_str(), needles)
			|| adapter_status_reports_same_corpus(suite.status, suite.evidence.as_str(), needles)
	}) || adapter.scenarios.iter().any(|scenario| {
		adapter_status_reports_same_corpus(scenario.status, scenario.scenario_id.as_str(), needles)
			|| adapter_status_reports_same_corpus(
				scenario.status,
				scenario.evidence.as_str(),
				needles,
			)
	})
}

fn adapter_status_reports_same_corpus(
	status: AdapterCoverageStatus,
	text: &str,
	needles: &[&str],
) -> bool {
	matches!(
		status,
		AdapterCoverageStatus::Pass
			| AdapterCoverageStatus::Real
			| AdapterCoverageStatus::WrongResult
			| AdapterCoverageStatus::LifecycleFail
	) && text_mentions_any(text, needles)
}

fn adapter_status_mentions_any(
	status: AdapterCoverageStatus,
	text: &str,
	needles: &[&str],
) -> bool {
	matches!(status, AdapterCoverageStatus::Pass | AdapterCoverageStatus::Real)
		&& text_mentions_any(text, needles)
}

fn text_mentions_any(text: &str, needles: &[&str]) -> bool {
	let text = text.to_ascii_lowercase();

	needles.iter().any(|needle| text.contains(&needle.to_ascii_lowercase()))
}
