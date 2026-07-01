use crate::{
	QuantitativePerQueryRow,
	quantitative::{audit_manifest::QuantitativeAuditEvidence, report::QuantitativeReportInput},
};

pub(in crate::quantitative::report::row) struct QuantitativeBenchmarkRowInput<'a, 'b> {
	pub(in crate::quantitative::report::row) input: &'a QuantitativeReportInput<'b>,
	pub(in crate::quantitative::report::row) corpus_id: &'a str,
	pub(in crate::quantitative::report::row) evidence_class: &'a str,
	pub(in crate::quantitative::report::row) per_query_rows: &'a [QuantitativePerQueryRow],
	pub(in crate::quantitative::report::row) ranking_query_count: usize,
	pub(in crate::quantitative::report::row) explicit_qrel_query_count: usize,
	pub(in crate::quantitative::report::row) metric_comparable: bool,
	pub(in crate::quantitative::report::row) result_state: &'a str,
	pub(in crate::quantitative::report::row) audit_evidence: QuantitativeAuditEvidence,
	pub(in crate::quantitative::report::row) leaderboard_eligible: bool,
}
