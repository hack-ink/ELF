use crate::{RealWorldReport, markdown, operational_reports::OperationalAuthorityRecoveryReport};

pub(super) fn render_markdown_operational_evidence(out: &mut String, report: &RealWorldReport) {
	let evidence = &report.operational_evidence;

	if evidence.schema.is_empty() {
		return;
	}

	out.push_str("## Operational Evidence Gates\n\n");
	out.push_str("This section separates operational evidence tiers so local fixture or public-proxy passes do not become private-corpus or provider-backed proof.\n\n");
	out.push_str(&format!("- Schema: `{}`\n", markdown::md_inline(evidence.schema.as_str())));
	out.push_str(&format!(
		"- Claim boundary: {}\n",
		markdown::md_cell(evidence.claim_boundary.as_str())
	));
	out.push_str(&format!(
		"- Missing private/provider inputs are typed blockers: `{}`\n",
		evidence.missing_private_provider_inputs_are_typed_blockers
	));
	out.push_str(&format!(
		"- Private-corpus pass claim allowed: `{}`\n",
		evidence.private_corpus_pass_claim_allowed
	));
	out.push_str(&format!(
		"- Provider-backed pass claim allowed: `{}`\n",
		evidence.provider_backed_pass_claim_allowed
	));
	out.push_str(&format!(
		"- Latency: `{}` measured job(s), `{}` missing, mean `{}`, max `{}`\n",
		evidence.latency.measured_job_count,
		evidence.latency.missing_latency_job_count,
		markdown::optional_f64(evidence.latency.mean_ms, " ms"),
		markdown::optional_f64(evidence.latency.max_ms, " ms")
	));
	out.push_str(&format!(
		"- Cost: `{}` job(s) reported cost, `{}` missing, `{}` zero-cost; total `{}`\n",
		evidence.cost.jobs_with_cost_report,
		evidence.cost.missing_cost_job_count,
		evidence.cost.zero_cost_job_count,
		markdown::cost_display(evidence.cost.total.as_ref())
	));
	out.push_str(&format!(
		"- Cost boundary: {}\n",
		markdown::md_cell(evidence.cost.claim_boundary.as_str())
	));
	out.push_str(&format!(
		"- Resource envelope jobs: `{}` total, `{}` pass; latency/resource dimensions `{}`\n",
		evidence.resource.resource_envelope_job_count,
		evidence.resource.resource_envelope_pass_count,
		evidence.resource.latency_resource_dimension_job_count
	));
	out.push_str(&format!(
		"- Cold-start/restore/rebuild: cold-start `{}`/`{}` pass, restore `{}`/`{}` pass, Qdrant rebuild `{}`/`{}` pass\n\n",
		evidence.cold_start_restore_rebuild.cold_start_pass_count,
		evidence.cold_start_restore_rebuild.cold_start_job_count,
		evidence.cold_start_restore_rebuild.restore_pass_count,
		evidence.cold_start_restore_rebuild.restore_job_count,
		evidence.cold_start_restore_rebuild.qdrant_rebuild_pass_count,
		evidence.cold_start_restore_rebuild.qdrant_rebuild_job_count
	));

	render_authority_recovery_summary(out, &evidence.authority_recovery);

	out.push_str("| Evidence Tier | Status | Jobs | Pass | Blocked | Incomplete | Not Encoded | Mean Latency | Cost | Resource | Cold Start | Restore | Qdrant Rebuild | Pass Claim |\n");
	out.push_str("| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | --- | ---: | ---: | ---: | ---: | --- |\n");

	for tier in &evidence.tiers {
		out.push_str(&format!(
			"| `{}` | `{}` | {} | {} | {} | {} | {} | `{}` | `{}` | {} | {} | {} | {} | `{}` |\n",
			markdown::md_inline(tier.tier.as_str()),
			markdown::status_str(tier.status),
			tier.job_count,
			tier.pass,
			tier.blocked,
			tier.incomplete,
			tier.not_encoded,
			markdown::optional_f64(tier.mean_latency_ms, " ms"),
			markdown::cost_display(tier.total_cost.as_ref()),
			tier.resource_evidence_count,
			tier.cold_start_evidence_count,
			tier.restore_evidence_count,
			tier.qdrant_rebuild_evidence_count,
			tier.pass_claim_allowed
		));
	}

	if evidence.tiers.iter().any(|tier| !tier.blocker_reasons.is_empty()) {
		out.push_str("\nTyped blocker reasons:\n");

		for tier in &evidence.tiers {
			for reason in &tier.blocker_reasons {
				out.push_str(&format!(
					"- `{}`: {}\n",
					markdown::md_inline(tier.tier.as_str()),
					markdown::md_cell(reason)
				));
			}
		}
	}

	out.push('\n');
}

fn render_authority_recovery_summary(
	out: &mut String,
	recovery: &OperationalAuthorityRecoveryReport,
) {
	out.push_str(&format!(
		"- Authority recovery drills: `{}`/`{}` pass, topology `{}`, failure injections `{}`, backup/PITR restored `{}`, degraded reads labeled `{}`, source-of-truth visible `{}`, RPO `{}`/`{}` met, RTO `{}`/`{}` met, record counts `{}`/`{}` preserved, source refs `{}`/`{}` preserved, lifecycle histories `{}`/`{}` preserved, idempotent replay `{}`, complete Qdrant rebuild `{}`, migration repair `{}`, dead-letter handled `{}`\n\n",
		recovery.drill_pass_count,
		recovery.drill_count,
		recovery.topology_reported_count,
		recovery.failure_injection_count,
		recovery.backup_pitr_restored_count,
		recovery.degraded_read_labeled_count,
		recovery.source_of_truth_visible_count,
		recovery.rpo_met_count,
		recovery.rpo_target_count,
		recovery.rto_met_count,
		recovery.rto_target_count,
		recovery.record_count_preserved_count,
		recovery.authority_plane_count,
		recovery.source_ref_preserved_count,
		recovery.authority_plane_count,
		recovery.lifecycle_history_preserved_count,
		recovery.authority_plane_count,
		recovery.idempotent_outbox_replay_count,
		recovery.qdrant_rebuild_complete_count,
		recovery.migration_repair_count,
		recovery.dead_letter_handled_count
	));
}
