use crate::{
	Result,
	quantitative::{
		self,
		audit_manifest::{self, QuantitativeAuditContext, QuantitativeAuditEvidence},
		report::QuantitativeReportInput,
	},
};

pub(super) struct QuantitativeAuditGates {
	pub(super) audit_evidence: QuantitativeAuditEvidence,
	pub(super) leaderboard_eligible: bool,
}

pub(super) fn quantitative_audit_gates(
	input: &QuantitativeReportInput<'_>,
	corpus_id: &str,
	evidence_class: &str,
	ranking_query_count: usize,
	explicit_qrel_query_count: usize,
	metric_comparable: bool,
) -> Result<QuantitativeAuditGates> {
	let audit_evidence = audit_manifest::quantitative_audit_evidence(
		input.audit_manifest_path,
		QuantitativeAuditContext {
			run_id: input.run_id,
			corpus_id,
			product: "ELF",
			adapter_id: input.adapter.adapter_id.as_str(),
			source_jobs: input.source_jobs,
			ranking_query_count,
			explicit_qrel_query_count,
		},
	)?;
	let leaderboard_eligible = quantitative::quantitative_row_leaderboard_eligible(
		evidence_class,
		input.source_jobs.len(),
		ranking_query_count,
		explicit_qrel_query_count,
		metric_comparable,
		&audit_evidence,
	);

	Ok(QuantitativeAuditGates { audit_evidence, leaderboard_eligible })
}
