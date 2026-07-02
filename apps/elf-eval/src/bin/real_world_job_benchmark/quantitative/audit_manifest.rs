mod artifacts;
mod evidence;
mod export;
mod validation;

pub(crate) use self::export::quantitative_audit_manifest_from_jobs;

use crate::{Path, RealWorldJob, Result};

pub(super) struct QuantitativeAuditContext<'a> {
	pub(super) run_id: &'a str,
	pub(super) corpus_id: &'a str,
	pub(super) product: &'a str,
	pub(super) adapter_id: &'a str,
	pub(super) source_jobs: &'a [RealWorldJob],
	pub(super) ranking_query_count: usize,
	pub(super) explicit_qrel_query_count: usize,
}

pub(super) struct QuantitativeAuditEvidence {
	pub(super) held_out: bool,
	pub(super) leakage_audited: bool,
	pub(super) audit_manifest_id: Option<String>,
}

pub(super) fn quantitative_audit_evidence(
	path: Option<&Path>,
	context: QuantitativeAuditContext<'_>,
) -> Result<QuantitativeAuditEvidence> {
	evidence::quantitative_audit_evidence(path, context)
}
