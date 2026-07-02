use crate::{Result, eyre};

pub(super) fn validate_audit_export_identity(product: &str, adapter_id: &str) -> Result<()> {
	if product.is_empty() || adapter_id.is_empty() {
		return Err(eyre::eyre!("quantitative audit export requires product and adapter_id."));
	}

	Ok(())
}
