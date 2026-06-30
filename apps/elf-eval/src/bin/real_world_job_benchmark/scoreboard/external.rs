mod narrative;
mod signals;
mod status;

use crate::scoreboard::{
	AdapterCoverageStatus, BTreeMap, ExternalAdapterReport, SCOREBOARD_RETRIEVAL_K,
	ScoreboardCoverageMetrics, ScoreboardMetrics, ScoreboardRetrievalMetrics, ScoreboardRow,
	common,
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
	let evidence_class = status::strongest_scoreboard_evidence_class(adapters);
	let result_state = status::external_project_result_state(adapters);
	let source_id_mapped = signals::external_project_source_id_mapped(adapters);
	let same_corpus = signals::external_project_same_corpus(adapters);
	let product_runtime =
		adapters.iter().any(|adapter| adapter.evidence_class == "live_real_world");
	let container_digest_identified =
		adapters.iter().any(|adapter| signals::adapter_has_container_digest(adapter));
	let typed_non_pass_count =
		adapters.iter().map(|adapter| status::adapter_typed_non_pass_count(adapter)).sum();
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
		strengths: narrative::external_project_strengths(adapters),
		weaknesses: narrative::external_project_weaknesses(adapters),
		next_evidence: Vec::new(),
		source_provenance: narrative::external_project_source_provenance(adapters),
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
