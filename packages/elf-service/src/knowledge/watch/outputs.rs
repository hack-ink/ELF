use crate::knowledge::watch::{
	BTreeMap, BTreeSet, KnowledgePageChangedSource, KnowledgePageRebuildOutput,
	KnowledgePageSection, KnowledgePageSourceRef, KnowledgeSourceKind, LintDraft, Uuid, Value,
	serde_json,
};

pub(in crate::knowledge) fn rebuild_outputs(
	sections: &[KnowledgePageSection],
	source_refs: &[KnowledgePageSourceRef],
	lint: &[LintDraft],
	diff: Option<&Value>,
	changed_sources: &[KnowledgePageChangedSource],
) -> Vec<KnowledgePageRebuildOutput> {
	let section_index = section_lookup(sections);
	let changed_keys = diff_section_keys(diff, "changed_section_keys");
	let mut outputs = lint_outputs(lint, &section_index);

	outputs.extend(changed_claim_outputs(sections, &changed_keys));
	outputs.extend(conflict_outputs(&outputs));
	outputs.extend(changed_source_outputs(source_refs, changed_sources));

	outputs
}

pub(in crate::knowledge) fn blocked_outputs(
	sections: &[KnowledgePageSection],
	lint: &[LintDraft],
	blocked_reason: &str,
) -> Vec<KnowledgePageRebuildOutput> {
	let section_index = section_lookup(sections);
	let mut outputs = lint_outputs(lint, &section_index);

	outputs.push(KnowledgePageRebuildOutput {
		output_type: "blocked".to_string(),
		severity: "error".to_string(),
		section_key: None,
		source_kind: None,
		source_id: None,
		message: "Knowledge page could not be rebuilt from its stored source refs.".to_string(),
		details: serde_json::json!({ "blocked_reason": blocked_reason }),
	});

	outputs
}

pub(in crate::knowledge) fn lint_outputs(
	lint: &[LintDraft],
	section_index: &BTreeMap<Uuid, (String, String)>,
) -> Vec<KnowledgePageRebuildOutput> {
	lint.iter().filter_map(|finding| lint_output(finding, section_index)).collect()
}

pub(in crate::knowledge) fn lint_output(
	finding: &LintDraft,
	section_index: &BTreeMap<Uuid, (String, String)>,
) -> Option<KnowledgePageRebuildOutput> {
	let output_type = match finding.finding_type.as_str() {
		"stale_source_ref" => "stale_section",
		"missing_citation" | "missing_source_ref" => "missing_citation",
		_ => return None,
	};
	let (section_key, heading) = finding
		.section_id
		.and_then(|section_id| section_index.get(&section_id))
		.cloned()
		.unwrap_or_else(|| ("page".to_string(), "Page".to_string()));

	Some(KnowledgePageRebuildOutput {
		output_type: output_type.to_string(),
		severity: finding.severity.clone(),
		section_key: Some(section_key.clone()),
		source_kind: finding.source_kind.map(KnowledgeSourceKind::as_str).map(ToString::to_string),
		source_id: finding.source_id,
		message: lint_output_message(output_type, heading.as_str()),
		details: serde_json::json!({
			"finding_type": finding.finding_type,
			"section_key": section_key,
			"lint_details": finding.details,
		}),
	})
}

pub(in crate::knowledge) fn changed_claim_outputs(
	sections: &[KnowledgePageSection],
	changed_keys: &BTreeSet<String>,
) -> Vec<KnowledgePageRebuildOutput> {
	sections
		.iter()
		.filter(|section| changed_keys.contains(section.section_key.as_str()))
		.map(|section| KnowledgePageRebuildOutput {
			output_type: "changed_claim".to_string(),
			severity: "info".to_string(),
			section_key: Some(section.section_key.clone()),
			source_kind: None,
			source_id: None,
			message: format!(
				"Knowledge page section '{}' changed after rebuilding from current sources.",
				section.heading
			),
			details: serde_json::json!({
				"section_key": section.section_key,
				"section_hash": section.content_hash,
			}),
		})
		.collect()
}

pub(in crate::knowledge) fn changed_source_outputs(
	source_refs: &[KnowledgePageSourceRef],
	changed_sources: &[KnowledgePageChangedSource],
) -> Vec<KnowledgePageRebuildOutput> {
	let changed = changed_source_set(changed_sources);

	source_refs
		.iter()
		.filter(|source_ref| {
			changed.contains(&(source_ref.source_kind.clone(), source_ref.source_id))
		})
		.map(|source_ref| KnowledgePageRebuildOutput {
			output_type: "changed_source".to_string(),
			severity: "info".to_string(),
			section_key: None,
			source_kind: Some(source_ref.source_kind.clone()),
			source_id: Some(source_ref.source_id),
			message: "Changed source is attached to this knowledge page.".to_string(),
			details: serde_json::json!({
				"source_kind": source_ref.source_kind,
				"source_id": source_ref.source_id,
				"section_id": source_ref.section_id,
			}),
		})
		.collect()
}

pub(in crate::knowledge) fn conflict_outputs(
	outputs: &[KnowledgePageRebuildOutput],
) -> Vec<KnowledgePageRebuildOutput> {
	let stale = output_section_keys(outputs, "stale_section");
	let changed = output_section_keys(outputs, "changed_claim");

	stale
		.intersection(&changed)
		.map(|section_key| {
			KnowledgePageRebuildOutput {
			output_type: "conflict".to_string(),
			severity: "warning".to_string(),
			section_key: Some(section_key.clone()),
			source_kind: None,
			source_id: None,
			message:
				"Stored derived section was stale and changed after rebuilding from current sources."
					.to_string(),
			details: serde_json::json!({
				"section_key": section_key,
				"reason": "stale_snapshot_changed_claim",
			}),
		}
		})
		.collect()
}

pub(in crate::knowledge) fn section_lookup(
	sections: &[KnowledgePageSection],
) -> BTreeMap<Uuid, (String, String)> {
	sections
		.iter()
		.map(|section| (section.section_id, (section.section_key.clone(), section.heading.clone())))
		.collect()
}

pub(in crate::knowledge) fn diff_section_keys(diff: Option<&Value>, key: &str) -> BTreeSet<String> {
	diff.and_then(|value| value.get(key))
		.and_then(Value::as_array)
		.map(|items| items.iter().filter_map(Value::as_str).map(ToString::to_string).collect())
		.unwrap_or_default()
}

pub(in crate::knowledge) fn changed_source_set(
	changed_sources: &[KnowledgePageChangedSource],
) -> BTreeSet<(String, Uuid)> {
	changed_sources
		.iter()
		.map(|source| (source.source_kind.as_str().to_string(), source.source_id))
		.collect()
}

pub(in crate::knowledge) fn output_section_keys(
	outputs: &[KnowledgePageRebuildOutput],
	output_type: &str,
) -> BTreeSet<String> {
	outputs
		.iter()
		.filter(|output| output.output_type == output_type)
		.filter_map(|output| output.section_key.clone())
		.collect()
}

pub(in crate::knowledge) fn candidate_reasons_by_section(
	outputs: &[KnowledgePageRebuildOutput],
) -> BTreeMap<&str, String> {
	let mut reasons = BTreeMap::<&str, String>::new();

	for output in outputs {
		let Some(section_key) = output.section_key.as_deref() else {
			continue;
		};

		match output.output_type.as_str() {
			"conflict" => {
				reasons.insert(section_key, "conflict".to_string());
			},
			"changed_claim" => {
				reasons.entry(section_key).or_insert_with(|| "changed_claim".to_string());
			},
			_ => {},
		}
	}

	reasons
}

pub(in crate::knowledge) fn lint_output_message(output_type: &str, heading: &str) -> String {
	match output_type {
		"stale_section" => {
			format!("Knowledge page section '{heading}' cites a stale or missing source.")
		},
		"missing_citation" => {
			format!("Knowledge page section '{heading}' is missing citation coverage.")
		},
		_ => format!("Knowledge page section '{heading}' needs operator review."),
	}
}
