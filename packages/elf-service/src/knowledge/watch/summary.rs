use crate::knowledge::watch::{
	KnowledgePageProposalRunSummary, KnowledgePageWatchRebuildItem,
	KnowledgePageWatchRebuildSummary,
};

pub(in crate::knowledge) fn watch_rebuild_summary(
	changed_source_count: usize,
	items: &[KnowledgePageWatchRebuildItem],
	memory_candidate_count: usize,
) -> KnowledgePageWatchRebuildSummary {
	KnowledgePageWatchRebuildSummary {
		changed_source_count,
		affected_page_count: items.len(),
		changed_page_count: items.iter().filter(|item| item.rebuild_state == "changed").count(),
		unchanged_page_count: items.iter().filter(|item| item.rebuild_state == "unchanged").count(),
		stale_page_count: items
			.iter()
			.filter(|item| item.outputs.iter().any(|output| output.output_type == "stale_section"))
			.count(),
		blocked_page_count: items.iter().filter(|item| item.rebuild_state == "blocked").count(),
		memory_candidate_count,
	}
}

pub(in crate::knowledge) fn watch_operator_summary(
	summary: &KnowledgePageWatchRebuildSummary,
	proposal_run: Option<&KnowledgePageProposalRunSummary>,
) -> Vec<String> {
	let mut out = vec![format!(
		"Changed-source rebuild inspected {} sources and {} affected knowledge pages.",
		summary.changed_source_count, summary.affected_page_count
	)];

	out.push(format!(
		"Page states: changed={}, unchanged={}, stale={}, blocked={}.",
		summary.changed_page_count,
		summary.unchanged_page_count,
		summary.stale_page_count,
		summary.blocked_page_count
	));
	out.push(format!(
		"Generated {} reviewable memory candidate proposals; source mutation remains disabled.",
		summary.memory_candidate_count
	));

	if let Some(run) = proposal_run {
		out.push(format!(
			"Queued consolidation run {} with {} proposal payloads for review.",
			run.run_id, run.proposal_count
		));
	}

	out
}

pub(in crate::knowledge) fn page_operator_summary(
	page_key: &str,
	rebuild_state: &str,
	output_count: usize,
	candidate_count: usize,
) -> String {
	format!(
		"Knowledge page '{page_key}' rebuild_state={rebuild_state}, outputs={output_count}, memory_candidates={candidate_count}."
	)
}
