use crate::ExportQuantitativeAuditManifestArgs;

pub(super) fn quantitative_audit_claim_boundary(
	args: &ExportQuantitativeAuditManifestArgs,
) -> String {
	args.claim_boundary.clone().unwrap_or_else(|| {
		if args.held_out || args.leakage_audited {
			concat!(
				"Audit manifest supplied by operator; runner validates run/corpus/product/",
				"adapter/count/query-id/artifact bindings before opening row gates."
			)
			.to_string()
		} else {
			concat!(
				"Diagnostic audit manifest binds the current product-runtime fixture set to ",
				"query ids and counts, but it does not prove held-out or leakage-audited status."
			)
			.to_string()
		}
	})
}
