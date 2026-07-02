use crate::{
	Path, QuantitativeBenchmarkRow, Result, eyre, quantitative::MIN_LEADERBOARD_QUERY_COUNT,
};

pub(super) fn validate_leaderboard_eligible_product_row(
	path: &Path,
	row: &QuantitativeBenchmarkRow,
) -> Result<()> {
	let has_audit_manifest_id = row
		.audit_manifest_id
		.as_deref()
		.is_some_and(|audit_manifest_id| !audit_manifest_id.trim().is_empty());

	if row.evidence_class != "live_real_world"
		|| row.sample_size < MIN_LEADERBOARD_QUERY_COUNT
		|| row.ranking_query_count != row.sample_size
		|| row.explicit_qrel_query_count != row.ranking_query_count
		|| !row.held_out
		|| !row.leakage_audited
		|| !has_audit_manifest_id
	{
		return Err(eyre::eyre!(
			"{} row {}:{} is marked leaderboard_eligible without the required live/product-runtime, query-count, explicit-qrel, held-out, leakage-audit, and audit-manifest controls.",
			path.display(),
			row.product,
			row.adapter_id
		));
	}

	Ok(())
}
