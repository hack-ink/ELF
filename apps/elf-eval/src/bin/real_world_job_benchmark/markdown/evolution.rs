use crate::markdown::{self, EvolutionJobReport, RealWorldReport};

pub(super) fn render_markdown_evolution(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Memory Evolution\n\n");
	out.push_str(&format!("- Stale answers: `{}`\n", report.evolution.stale_answer_count));
	out.push_str(&format!(
		"- Conflict detections: `{}`\n",
		report.evolution.conflict_detection_count
	));
	out.push_str(&format!(
		"- Update rationales available: `{}`\n",
		report.evolution.update_rationale_available_count
	));
	out.push_str(&format!(
		"- Temporal validity not encoded: `{}`\n\n",
		report.evolution.temporal_validity_not_encoded_count
	));
	out.push_str(&format!(
		"- History readback encoded: `{}`\n\n",
		report.evolution.history_readback_encoded_count
	));
	out.push_str("| Suite | Job | Current Evidence | Historical Evidence | Tombstone/Invalidation | Selected Current | Selected Historical | Selected Rationale | Selected Tombstone/Invalidation | Selected But Not Narrated | Stale Traps Used | Conflict Count | Detected | Update Rationale | Temporal Validity | History Readback | Follow-up |\n");
	out.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | ---: | ---: | --- | --- | --- | --- |\n");

	for job in &report.jobs {
		let Some(evolution) = &job.evolution else {
			continue;
		};

		out.push_str(&format!(
			"| {} | {} | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | {} | {} | `{}` | `{}` | `{}` | {} |\n",
			markdown::md_cell(job.suite_id.as_str()),
			markdown::md_cell(job.job_id.as_str()),
			markdown::md_inline(evolution.current_evidence.join(", ").as_str()),
			markdown::md_inline(evolution.historical_evidence.join(", ").as_str()),
			markdown::md_inline(
				evolution
					.tombstone_evidence
					.iter()
					.chain(evolution.invalidation_evidence.iter())
					.cloned()
					.collect::<Vec<_>>()
					.join(", ")
					.as_str()
			),
			markdown::md_inline(evolution.selected_current_evidence.join(", ").as_str()),
			markdown::md_inline(evolution.selected_historical_evidence.join(", ").as_str()),
			markdown::md_inline(evolution.selected_rationale_evidence.join(", ").as_str()),
			markdown::md_inline(
				evolution
					.selected_tombstone_evidence
					.iter()
					.chain(evolution.selected_invalidation_evidence.iter())
					.cloned()
					.collect::<Vec<_>>()
					.join(", ")
					.as_str()
			),
			markdown::md_inline(evolution.selected_but_not_narrated_evidence.join(", ").as_str()),
			markdown::md_inline(evolution.stale_trap_ids_used.join(", ").as_str()),
			evolution.conflict_count,
			evolution.conflict_detection_count,
			markdown::bool_display(evolution.update_rationale_available),
			temporal_display(evolution),
			history_display(evolution),
			markdown::md_cell(evolution.follow_up.as_deref().unwrap_or("-"))
		));
	}

	out.push('\n');
}

fn temporal_display(evolution: &EvolutionJobReport) -> &'static str {
	if evolution.temporal_validity_not_encoded {
		"not_encoded"
	} else if evolution.temporal_validity_encoded {
		"encoded"
	} else if evolution.temporal_validity_required {
		"required"
	} else {
		"-"
	}
}

fn history_display(evolution: &EvolutionJobReport) -> String {
	if !evolution.history_readback_encoded {
		return "-".to_string();
	}

	let mut parts = vec![format!("events={}", evolution.history_event_types.join(","))];

	if evolution.history_requires_note_version_links {
		parts.push("note_version_links=true".to_string());
	}

	parts.join(";")
}
