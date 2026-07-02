use crate::{ExportQuantitativeProductManifestArgs, Result, eyre};

pub(super) fn validate_export_identity(
	args: &ExportQuantitativeProductManifestArgs,
	product: &str,
	adapter_id: &str,
	adapter_name: &str,
) -> Result<()> {
	if product.is_empty() || adapter_id.is_empty() || adapter_name.is_empty() {
		return Err(eyre::eyre!(
			"{} cannot export an incomplete quantitative product identity.",
			args.report.display()
		));
	}
	if product == "ELF" {
		return Err(eyre::eyre!(
			"{} exports product ELF; use --product for external product manifest exports.",
			args.report.display()
		));
	}

	Ok(())
}
