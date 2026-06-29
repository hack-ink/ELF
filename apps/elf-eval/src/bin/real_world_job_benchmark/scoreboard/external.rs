use crate::scoreboard::{
	self, AdapterCoverageStatus, BTreeMap, BTreeSet, ExternalAdapterReport, SCOREBOARD_RETRIEVAL_K,
	ScenarioComparisonOutcome, ScoreboardCoverageMetrics, ScoreboardMetrics,
	ScoreboardRetrievalMetrics, ScoreboardRow, common,
};

pub(super) fn external_project_scoreboard_rows(
	adapters: &[ExternalAdapterReport],
) -> Vec<ScoreboardRow> {
	let mut by_project: BTreeMap<String, Vec<&ExternalAdapterReport>> = BTreeMap::new();

	for adapter in adapters.iter().filter(|adapter| adapter.project != "ELF") {
		by_project.entry(adapter.project.clone()).or_default().push(adapter);
	}

	by_project
		.into_iter()
		.map(|(project, adapters)| external_project_scoreboard_row(project, adapters.as_slice()))
		.collect()
}

fn external_project_scoreboard_row(
	project: String,
	adapters: &[&ExternalAdapterReport],
) -> ScoreboardRow {
	let evidence_class = strongest_scoreboard_evidence_class(adapters);
	let result_state = external_project_result_state(adapters);
	let source_id_mapped = external_project_source_id_mapped(adapters);
	let same_corpus = external_project_same_corpus(adapters);
	let product_runtime =
		adapters.iter().any(|adapter| adapter.evidence_class == "live_real_world");
	let container_digest_identified =
		adapters.iter().any(|adapter| adapter_has_container_digest(adapter));
	let typed_non_pass_count =
		adapters.iter().map(|adapter| adapter_typed_non_pass_count(adapter)).sum();
	let mut row = ScoreboardRow {
		product_id: scoreboard_project_id(project.as_str()),
		product_name: project,
		row_source: "external_adapter_manifest".to_string(),
		evidence_class: evidence_class.clone(),
		result_state,
		comparable: false,
		same_corpus,
		source_id_mapped,
		held_out: false,
		leakage_audited: false,
		product_runtime,
		container_digest_identified,
		metrics: external_project_scoreboard_metrics(
			adapters,
			evidence_class.as_str(),
			typed_non_pass_count,
		),
		strengths: external_project_strengths(adapters),
		weaknesses: external_project_weaknesses(adapters),
		next_evidence: Vec::new(),
		source_provenance: external_project_source_provenance(adapters),
	};

	common::scoreboard_apply_comparability_gaps(&mut row);

	row
}

fn external_project_scoreboard_metrics(
	adapters: &[&ExternalAdapterReport],
	evidence_class: &str,
	typed_non_pass_count: usize,
) -> ScoreboardMetrics {
	let pass_count = adapters
		.iter()
		.flat_map(|adapter| adapter.suites.iter())
		.filter(|suite| suite.status == AdapterCoverageStatus::Pass)
		.count();
	let suite_count = adapters.iter().map(|adapter| adapter.suites.len()).sum();

	ScoreboardMetrics {
		retrieval: ScoreboardRetrievalMetrics {
			k: SCOREBOARD_RETRIEVAL_K,
			metric_basis: "external_adapter_manifest_no_ordered_evidence".to_string(),
			..ScoreboardRetrievalMetrics::default()
		},
		coverage: ScoreboardCoverageMetrics {
			job_count: 0,
			encoded_suite_count: suite_count,
			pass_count,
			typed_non_pass_count,
			source_ref_coverage: None,
			evidence_coverage: None,
			evidence_class: evidence_class.to_string(),
		},
		..ScoreboardMetrics::default()
	}
}

fn strongest_scoreboard_evidence_class(adapters: &[&ExternalAdapterReport]) -> String {
	for evidence_class in ["live_real_world", "live_baseline", "fixture_backed", "research_gate"] {
		if adapters.iter().any(|adapter| {
			common::scoreboard_evidence_class(adapter.evidence_class.as_str()) == evidence_class
		}) {
			return evidence_class.to_string();
		}
	}

	"research_gate".to_string()
}

fn external_project_result_state(adapters: &[&ExternalAdapterReport]) -> String {
	for status in [
		AdapterCoverageStatus::WrongResult,
		AdapterCoverageStatus::Blocked,
		AdapterCoverageStatus::Incomplete,
		AdapterCoverageStatus::LifecycleFail,
		AdapterCoverageStatus::NotEncoded,
		AdapterCoverageStatus::Unsupported,
	] {
		if adapters.iter().any(|adapter| adapter_has_status(adapter, status)) {
			return adapter_status_to_scoreboard_state(status).to_string();
		}
	}

	"not_comparable".to_string()
}

fn adapter_has_status(adapter: &ExternalAdapterReport, status: AdapterCoverageStatus) -> bool {
	adapter.overall_status == status
		|| adapter.setup.status == status
		|| adapter.run.status == status
		|| adapter.result.status == status
		|| adapter.capabilities.iter().any(|capability| capability.status == status)
		|| adapter.suites.iter().any(|suite| suite.status == status)
		|| adapter.scenarios.iter().any(|scenario| scenario.status == status)
}

fn external_project_same_corpus(adapters: &[&ExternalAdapterReport]) -> bool {
	let needles = &["same-corpus", "same corpus", "same_corpus", "shared corpus"];

	adapters.iter().any(|adapter| {
		text_mentions_any(adapter.adapter_kind.as_str(), needles)
			|| adapter_has_reported_same_corpus_text(adapter, needles)
	})
}

fn external_project_source_id_mapped(adapters: &[&ExternalAdapterReport]) -> bool {
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

fn adapter_status_to_scoreboard_state(status: AdapterCoverageStatus) -> &'static str {
	match status {
		AdapterCoverageStatus::WrongResult | AdapterCoverageStatus::LifecycleFail => "wrong_result",
		AdapterCoverageStatus::Blocked => "blocked",
		AdapterCoverageStatus::Incomplete => "incomplete",
		AdapterCoverageStatus::NotEncoded | AdapterCoverageStatus::Unsupported => "not_encoded",
		AdapterCoverageStatus::Real
		| AdapterCoverageStatus::Mocked
		| AdapterCoverageStatus::Pass => "not_comparable",
	}
}

fn adapter_typed_non_pass_count(adapter: &ExternalAdapterReport) -> usize {
	let direct_statuses =
		[adapter.overall_status, adapter.setup.status, adapter.run.status, adapter.result.status];
	let direct = direct_statuses
		.into_iter()
		.filter(|status| adapter_status_is_typed_non_pass(*status))
		.count();
	let capability = adapter
		.capabilities
		.iter()
		.filter(|capability| adapter_status_is_typed_non_pass(capability.status))
		.count();
	let suites = adapter
		.suites
		.iter()
		.filter(|suite| adapter_status_is_typed_non_pass(suite.status))
		.count();
	let scenarios = adapter
		.scenarios
		.iter()
		.filter(|scenario| adapter_status_is_typed_non_pass(scenario.status))
		.count();

	direct + capability + suites + scenarios
}

fn adapter_status_is_typed_non_pass(status: AdapterCoverageStatus) -> bool {
	matches!(
		status,
		AdapterCoverageStatus::Unsupported
			| AdapterCoverageStatus::Blocked
			| AdapterCoverageStatus::Incomplete
			| AdapterCoverageStatus::WrongResult
			| AdapterCoverageStatus::LifecycleFail
			| AdapterCoverageStatus::NotEncoded
	)
}

fn adapter_has_container_digest(adapter: &ExternalAdapterReport) -> bool {
	adapter.setup.evidence.contains("sha256:")
		|| adapter.run.evidence.contains("sha256:")
		|| adapter.result.evidence.contains("sha256:")
		|| adapter.evidence.iter().any(|evidence| {
			evidence.reference.contains("sha256:") || evidence.reference.contains("digest")
		})
}

fn external_project_strengths(adapters: &[&ExternalAdapterReport]) -> Vec<String> {
	let mut strengths = BTreeSet::new();

	for adapter in adapters {
		for capability in &adapter.capabilities {
			if matches!(
				capability.status,
				AdapterCoverageStatus::Pass | AdapterCoverageStatus::Real
			) {
				strengths.insert(format!(
					"{} capability is {}.",
					capability.capability,
					scoreboard::adapter_status_str(capability.status)
				));
			}
		}
		for scenario in &adapter.scenarios {
			if scoreboard::scenario_comparison_outcome(scenario) == ScenarioComparisonOutcome::Loss
			{
				strengths.insert(format!(
					"Scenario {} is recorded as a competitor strength.",
					scenario.scenario_id
				));
			}
		}
	}

	strengths.into_iter().take(6).collect()
}

fn external_project_weaknesses(adapters: &[&ExternalAdapterReport]) -> Vec<String> {
	let mut weaknesses = BTreeSet::new();

	for adapter in adapters {
		if adapter.overall_status != AdapterCoverageStatus::Pass {
			weaknesses.insert(format!(
				"Adapter {} overall status is {}.",
				adapter.adapter_id,
				scoreboard::adapter_status_str(adapter.overall_status)
			));
		}

		for suite in &adapter.suites {
			if adapter_status_is_typed_non_pass(suite.status) {
				weaknesses.insert(format!(
					"Suite {} is {}.",
					suite.suite_id,
					scoreboard::adapter_status_str(suite.status)
				));
			}
		}
	}

	weaknesses.into_iter().take(8).collect()
}

fn external_project_source_provenance(adapters: &[&ExternalAdapterReport]) -> Vec<String> {
	let mut provenance = BTreeSet::new();

	for adapter in adapters {
		for evidence in &adapter.evidence {
			provenance.insert(evidence.reference.clone());
		}
		for artifact in [&adapter.setup.artifact, &adapter.run.artifact, &adapter.result.artifact]
			.into_iter()
			.flatten()
		{
			provenance.insert(artifact.clone());
		}
	}

	provenance.into_iter().take(12).collect()
}

fn scoreboard_project_id(project: &str) -> String {
	project
		.chars()
		.map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { '_' })
		.collect::<String>()
		.split('_')
		.filter(|part| !part.is_empty())
		.collect::<Vec<_>>()
		.join("_")
}
