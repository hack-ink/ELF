use crate::{AdapterReport, JobReport, Path, RealWorldJob, ReportSummary};

pub(crate) struct QuantitativeReportInput<'a> {
	pub(crate) run_id: &'a str,
	pub(crate) generated_at: &'a str,
	pub(crate) adapter: &'a AdapterReport,
	pub(crate) source_jobs: &'a [RealWorldJob],
	pub(crate) jobs: &'a [JobReport],
	pub(crate) summary: &'a ReportSummary,
	pub(crate) product_manifest_path: Option<&'a Path>,
	pub(crate) audit_manifest_path: Option<&'a Path>,
}
