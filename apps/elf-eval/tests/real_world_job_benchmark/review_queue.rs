use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::{dreaming_reports, support};

#[test]
fn dreaming_review_queue_report_wires_reviewable_policy_contract() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::dreaming_review_queue_report_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(support::dreaming_review_queue_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;
	let workspace = support::workspace_root()?;
	let service = dreaming_reports::read_rust_module_sources(
		&workspace.join("packages/elf-service/src"),
		"dreaming_review_queue",
	)?;
	let service_lib = fs::read_to_string(workspace.join("packages/elf-service/src/lib.rs"))?;
	let routes =
		dreaming_reports::read_rust_module_sources(&workspace.join("apps/elf-api/src"), "routes")?;
	let mcp = dreaming_reports::read_rust_module_sources(
		&workspace.join("apps/elf-mcp/src/app"),
		"server",
	)?;
	let consolidation_spec =
		fs::read_to_string(workspace.join("docs/spec/system_consolidation_proposals_v1.md"))?;
	let service_spec =
		fs::read_to_string(workspace.join("docs/spec/system_elf_memory_service_v2.md"))?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.dreaming_review_queue_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-1021"));
	assert_eq!(
		report.pointer("/summary/queue_schema").and_then(Value::as_str),
		Some("elf.dreaming_review_queue/v1")
	);
	assert_eq!(
		report.pointer("/summary/source_mutation_allowed").and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		report.pointer("/summary/high_impact_requires_review").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(report.pointer("/summary/variant_count").and_then(Value::as_u64), Some(9));

	for suite in ["memory_summary", "proactive_brief", "scheduled_memory", "consolidation"] {
		assert!(support::array_contains_str(&report, "/summary/covered_existing_suites", suite)?);
	}
	for variant in
		["tag", "duplicate_merge", "page_rebuild", "memory_promotion", "graph_fact", "correction"]
	{
		assert!(support::array_contains_str(&report, "/summary/covered_future_variants", variant)?);

		support::find_by_field(
			support::array_at(&report, "/queue_variants")?,
			"/variant",
			variant,
		)?;
	}
	for field in [
		"source_refs",
		"affected_refs",
		"confidence",
		"unsupported_claim_flags",
		"diff",
		"policy",
		"review_audit",
	] {
		assert!(support::array_contains_str(&report, "/required_item_fields", field)?);
	}

	assert!(service.contains("ELF_DREAMING_REVIEW_QUEUE_SCHEMA_V1"));
	assert!(service.contains("pub async fn dreaming_review_queue"));
	assert!(service.contains("source_mutation_allowed: false"));
	assert!(service.contains("low_risk_derived_organization"));
	assert!(service.contains("available_review_actions"));
	assert!(service_lib.contains("pub mod dreaming_review_queue"));
	assert!(service_lib.contains("DreamingReviewQueueResponse"));
	assert!(routes.contains("/v2/admin/dreaming/review-queue"));
	assert!(routes.contains("DreamingReviewQueueRequest"));
	assert!(routes.contains("async fn dreaming_review_queue"));
	assert!(mcp.contains("elf_dreaming_review_queue"));
	assert!(mcp.contains("dreaming_review_queue_schema"));
	assert!(mcp.contains("/v2/admin/dreaming/review-queue"));
	assert!(consolidation_spec.contains("elf.dreaming_review_queue/v1"));
	assert!(consolidation_spec.contains("source_mutation_allowed"));
	assert!(consolidation_spec.contains("duplicate_merge"));
	assert!(service_spec.contains("GET /v2/admin/dreaming/review-queue"));
	assert!(service_spec.contains("source refs, affected refs, confidence"));
	assert!(markdown.contains("Dreaming Review Queue Report"));
	assert!(markdown.contains("Auto-apply is limited to approved low-risk"));
	assert!(benchmarking_index.contains("2026-06-20-dreaming-review-queue-report.md"));
	assert!(readme.contains("Dreaming review queue after XY-1021"));
	assert!(readme.contains("elf.dreaming_review_queue/v1"));

	Ok(())
}
