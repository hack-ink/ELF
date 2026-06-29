use crate::{
	HashMap, IngestedCorpus, KnowledgeMaterializationEvidence, KnowledgePageLintResponse,
	KnowledgePageResponse, LoadedJob, Result, Uuid, serde_json,
};

pub(super) fn knowledge_page_artifact(
	loaded: &LoadedJob,
	ingested: &IngestedCorpus,
	first: &KnowledgePageResponse,
	second: &KnowledgePageResponse,
	lint: &KnowledgePageLintResponse,
) -> Result<serde_json::Value> {
	let reverse = note_id_to_evidence_id(ingested);
	let mut sections = second
		.sections
		.iter()
		.map(|section| {
			let evidence_ids = section
				.source_backlinks
				.iter()
				.filter_map(|source| reverse.get(&source.source_id).cloned())
				.collect::<Vec<_>>();

			serde_json::json!({
				"section_id": section.section_key.clone(),
				"heading": section.heading.clone(),
				"role": section.role.clone(),
				"content": section.content.clone(),
				"evidence_ids": evidence_ids,
				"timeline_event_ids": []
			})
		})
		.collect::<Vec<_>>();

	sections.extend(unsupported_sections_from_fixture(loaded));

	Ok(serde_json::json!({
		"page_id": second.page.page_id.to_string(),
		"page_type": second.page.page_kind.clone(),
		"title": second.page.title.clone(),
		"sections": sections,
		"backlinks": source_backlinks(ingested),
		"lint_findings": lint_findings_for_page(loaded, ingested, lint),
		"page_version_diff": second.page.previous_version_diff.clone(),
		"rebuild": {
			"first_hash": first.page.content_hash.clone(),
			"second_hash": second.page.content_hash.clone(),
			"deterministic": first.page.content_hash == second.page.content_hash,
			"allowed_variance": []
		}
	}))
}

pub(super) fn knowledge_materialization_evidence(
	page: &KnowledgePageResponse,
	lint: &KnowledgePageLintResponse,
	search_result_count: usize,
) -> KnowledgeMaterializationEvidence {
	let unsupported_claim_count =
		lint.findings.iter().filter(|finding| finding.finding_type == "unsupported_claim").count()
			+ page.sections.iter().filter(|section| section.unsupported_reason.is_some()).count();

	KnowledgeMaterializationEvidence {
		page_ids: vec![page.page.page_id],
		search_result_count,
		lint_finding_count: lint.findings.len(),
		stale_source_finding_count: lint
			.findings
			.iter()
			.filter(|finding| finding.finding_type == "stale_source_ref")
			.count(),
		unsupported_claim_count,
		citation_count: page.sections.iter().map(|section| section.citation_count).sum(),
		source_ref_count: page.source_refs.len(),
		version_diff_available: page
			.page
			.previous_version_diff
			.as_ref()
			.and_then(|diff| diff.get("available"))
			.and_then(serde_json::Value::as_bool)
			.unwrap_or(false),
	}
}

pub(super) fn stale_trap_evidence_ids(loaded: &LoadedJob) -> Vec<String> {
	loaded
		.value
		.get("negative_traps")
		.and_then(serde_json::Value::as_array)
		.into_iter()
		.flatten()
		.filter(|trap| {
			trap.get("type").and_then(serde_json::Value::as_str) == Some("stale_fact")
				&& trap.get("failure_if_used").and_then(serde_json::Value::as_bool).unwrap_or(false)
		})
		.flat_map(|trap| {
			trap.get("evidence_ids")
				.and_then(serde_json::Value::as_array)
				.into_iter()
				.flatten()
				.filter_map(serde_json::Value::as_str)
				.map(ToString::to_string)
				.collect::<Vec<_>>()
		})
		.collect()
}

fn note_id_to_evidence_id(ingested: &IngestedCorpus) -> HashMap<Uuid, String> {
	let mut out = HashMap::new();

	for (evidence_id, note_ids) in &ingested.note_ids_by_evidence {
		for note_id in note_ids {
			out.insert(*note_id, evidence_id.clone());
		}
	}

	out
}

fn source_backlinks(ingested: &IngestedCorpus) -> Vec<String> {
	let mut backlinks = ingested
		.note_ids_by_evidence
		.keys()
		.map(|evidence_id| format!("source:{evidence_id}"))
		.collect::<Vec<_>>();

	backlinks.sort();

	backlinks
}

fn lint_findings_for_page(
	loaded: &LoadedJob,
	ingested: &IngestedCorpus,
	lint: &KnowledgePageLintResponse,
) -> Vec<serde_json::Value> {
	let reverse = note_id_to_evidence_id(ingested);

	lint.findings
		.iter()
		.map(|finding| {
			let evidence_ids = finding
				.source_id
				.and_then(|source_id| reverse.get(&source_id).cloned())
				.into_iter()
				.collect::<Vec<_>>();
			let trap_id = evidence_ids
				.first()
				.and_then(|evidence_id| trap_id_for_evidence(loaded, evidence_id));

			serde_json::json!({
				"finding_id": finding.finding_id.to_string(),
				"finding_type": finding.finding_type.clone(),
				"severity": finding.severity.clone(),
				"text": finding.message.clone(),
				"evidence_ids": evidence_ids,
				"trap_id": trap_id
			})
		})
		.collect()
}

fn unsupported_sections_from_fixture(loaded: &LoadedJob) -> Vec<serde_json::Value> {
	let Some(pages) = loaded
		.value
		.pointer("/corpus/adapter_response/answer/pages")
		.and_then(serde_json::Value::as_array)
	else {
		return Vec::new();
	};
	let mut sections = Vec::new();

	for page in pages {
		let Some(page_sections) = page.get("sections").and_then(serde_json::Value::as_array) else {
			continue;
		};

		for section in page_sections {
			let Some(reason) =
				section.get("unsupported_reason").and_then(serde_json::Value::as_str)
			else {
				continue;
			};

			sections.push(serde_json::json!({
				"section_id": section
					.get("section_id")
					.and_then(serde_json::Value::as_str)
					.unwrap_or("unsupported-summary"),
				"heading": section
					.get("heading")
					.and_then(serde_json::Value::as_str)
					.unwrap_or("Unsupported Summary"),
				"role": section.get("role").and_then(serde_json::Value::as_str).unwrap_or("summary"),
				"content": section.get("content").and_then(serde_json::Value::as_str).unwrap_or(reason),
				"evidence_ids": [],
				"timeline_event_ids": [],
				"unsupported_reason": reason
			}));
		}
	}

	sections
}

fn trap_id_for_evidence(loaded: &LoadedJob, evidence_id: &str) -> Option<String> {
	loaded
		.value
		.get("negative_traps")
		.and_then(serde_json::Value::as_array)?
		.iter()
		.find(|trap| {
			trap.get("evidence_ids")
				.and_then(serde_json::Value::as_array)
				.is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(evidence_id)))
		})
		.and_then(|trap| trap.get("trap_id").and_then(serde_json::Value::as_str))
		.map(ToString::to_string)
}
