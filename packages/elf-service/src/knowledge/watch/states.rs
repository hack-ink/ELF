use crate::knowledge::watch::{
	BTreeMap, KnowledgePageRebuildOutput, KnowledgePageSection, KnowledgePageSectionRebuildState,
	KnowledgePageSectionResponse, Value, outputs,
};

pub(in crate::knowledge) fn successful_section_states(
	before_sections: &[KnowledgePageSection],
	rebuilt_sections: &[KnowledgePageSectionResponse],
	outputs: &[KnowledgePageRebuildOutput],
) -> Vec<KnowledgePageSectionRebuildState> {
	let output_map = outputs_by_section(outputs);
	let before_by_key = before_sections
		.iter()
		.map(|section| (section.section_key.as_str(), section))
		.collect::<BTreeMap<_, _>>();

	rebuilt_sections
		.iter()
		.map(|section| {
			let output_types =
				output_map.get(section.section_key.as_str()).cloned().unwrap_or_default();
			let lint_finding_types = lint_finding_types_for_outputs(&output_types);
			let state = section_state(
				before_by_key.get(section.section_key.as_str()).copied(),
				section,
				&output_types,
			);

			KnowledgePageSectionRebuildState {
				section_key: section.section_key.clone(),
				heading: section.heading.clone(),
				state,
				output_types,
				lint_finding_types,
			}
		})
		.collect()
}

pub(in crate::knowledge) fn blocked_section_states(
	sections: &[KnowledgePageSection],
	outputs: &[KnowledgePageRebuildOutput],
) -> Vec<KnowledgePageSectionRebuildState> {
	let output_map = outputs_by_section(outputs);

	sections
		.iter()
		.map(|section| {
			let output_types =
				output_map.get(section.section_key.as_str()).cloned().unwrap_or_default();
			let lint_finding_types = lint_finding_types_for_outputs(&output_types);
			let state = if output_types.iter().any(|kind| kind == "missing_citation") {
				"blocked"
			} else if output_types.iter().any(|kind| kind == "stale_section") {
				"stale"
			} else {
				"blocked"
			};

			KnowledgePageSectionRebuildState {
				section_key: section.section_key.clone(),
				heading: section.heading.clone(),
				state: state.to_string(),
				output_types,
				lint_finding_types,
			}
		})
		.collect()
}

pub(in crate::knowledge) fn successful_rebuild_state(
	diff: Option<&Value>,
	outputs: &[KnowledgePageRebuildOutput],
) -> String {
	if diff_content_changed(diff) {
		return "changed".to_string();
	}

	if outputs.iter().any(|output| output.output_type == "stale_section") {
		return "stale".to_string();
	}

	"unchanged".to_string()
}

fn section_state(
	before: Option<&KnowledgePageSection>,
	after: &KnowledgePageSectionResponse,
	output_types: &[String],
) -> String {
	if output_types.iter().any(|kind| kind == "missing_citation") {
		return "blocked".to_string();
	}
	if before.is_some_and(|section| section.content_hash != after.content_hash)
		|| output_types.iter().any(|kind| kind == "changed_claim" || kind == "conflict")
	{
		return "changed".to_string();
	}

	if output_types.iter().any(|kind| kind == "stale_section") {
		return "stale".to_string();
	}

	"unchanged".to_string()
}

fn diff_content_changed(diff: Option<&Value>) -> bool {
	diff.and_then(|value| value.get("content_changed")).and_then(Value::as_bool).unwrap_or(false)
		|| !outputs::diff_section_keys(diff, "added_section_keys").is_empty()
		|| !outputs::diff_section_keys(diff, "removed_section_keys").is_empty()
		|| !outputs::diff_section_keys(diff, "changed_section_keys").is_empty()
}

fn outputs_by_section(outputs: &[KnowledgePageRebuildOutput]) -> BTreeMap<&str, Vec<String>> {
	let mut map = BTreeMap::<&str, Vec<String>>::new();

	for output in outputs {
		let Some(section_key) = output.section_key.as_deref() else {
			continue;
		};

		map.entry(section_key).or_default().push(output.output_type.clone());
	}
	for values in map.values_mut() {
		values.sort();
		values.dedup();
	}

	map
}

fn lint_finding_types_for_outputs(output_types: &[String]) -> Vec<String> {
	let mut out = output_types
		.iter()
		.filter_map(|output_type| match output_type.as_str() {
			"stale_section" => Some("stale_source_ref".to_string()),
			"missing_citation" => Some("missing_citation".to_string()),
			_ => None,
		})
		.collect::<Vec<_>>();

	out.sort();
	out.dedup();

	out
}
