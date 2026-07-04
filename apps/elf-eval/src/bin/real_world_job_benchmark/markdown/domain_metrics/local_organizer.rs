use crate::markdown::{self, RealWorldReport};

pub(in crate::markdown) fn render_markdown_local_organizer(
	out: &mut String,
	report: &RealWorldReport,
) {
	let Some(summary) = &report.summary.local_organizer else {
		return;
	};

	out.push_str("## Local Background Organizer\n\n");
	out.push_str("| Jobs | Extraction F1 Not Encoded | Schema Valid | Citation/Source-Ref Coverage | Unsupported Claim Rate | Stale/Correction/Delete | Source Mutations | Silent Memory Authority Mutations | Escalation Rate | Mean Latency | P95 Latency |\n");
	out.push_str(
		"| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
	);
	out.push_str(&format!(
		"| {} | {} | {} | `{}` | `{}` | `{}` | {} | {} | `{}` | `{}` | `{}` |\n\n",
		summary.job_count,
		summary.extraction_f1_not_encoded_count,
		summary.json_schema_valid_count,
		markdown::round3(summary.citation_source_ref_coverage),
		markdown::round3(summary.unsupported_claim_rate),
		markdown::round3(summary.stale_correction_delete_score),
		summary.source_mutation_count,
		summary.silent_memory_authority_mutation_count,
		markdown::round3(summary.escalation_rate),
		markdown::optional_f64(summary.mean_latency_ms, " ms"),
		markdown::optional_f64(summary.p95_latency_ms, " ms"),
	));
	out.push_str("Local organizer outputs are proposal-only benchmark evidence; passing this slice does not authorize small/local models to write Memory Authority records.\n\n");
	out.push_str("### Model Tiers\n\n");
	out.push_str("| Tier | Jobs | Mean Latency | P95 Latency | Cost | Resource Footprints |\n");
	out.push_str("| --- | ---: | ---: | ---: | --- | --- |\n");

	for tier in &summary.tiers {
		out.push_str(&format!(
			"| {} | {} | `{}` | `{}` | {} | {} |\n",
			markdown::md_cell(tier.model_tier.as_str()),
			tier.job_count,
			markdown::optional_f64(tier.mean_latency_ms, " ms"),
			markdown::optional_f64(tier.p95_latency_ms, " ms"),
			markdown::cost_display(tier.total_cost.as_ref()),
			markdown::md_cell(&format!("{} sample(s)", tier.resource_footprints.len())),
		));
	}

	out.push('\n');
}
