use crate::markdown::{self, RealWorldReport};

pub(in crate::markdown) fn render_markdown_knowledge(out: &mut String, report: &RealWorldReport) {
	let knowledge_jobs =
		report.jobs.iter().filter(|job| job.knowledge.is_some()).collect::<Vec<_>>();

	if knowledge_jobs.is_empty() {
		return;
	}

	out.push_str("## Knowledge Page Metrics\n\n");
	out.push_str("| Job | Pages | Sections | Citation Coverage | Stale Claim Detection | Rebuild Determinism | Version Diff Coverage | Page Usefulness | Backlinks | Unsupported Summaries | Untraced Sections | Allowed Variance |\n");
	out.push_str(
		"| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
	);

	for job in knowledge_jobs {
		let Some(knowledge) = &job.knowledge else {
			continue;
		};

		out.push_str(&format!(
			"| {} | {} | {} | `{:.3}` | `{:.3}` | `{:.3}` | `{:.3}` | `{:.3}` | {} | {} | {} | {} |\n",
			markdown::md_cell(job.job_id.as_str()),
			knowledge.page_count,
			knowledge.section_count,
			knowledge.citation_coverage,
			knowledge.stale_claim_detection,
			knowledge.rebuild_determinism,
			knowledge.version_diff_coverage,
			knowledge.page_usefulness,
			knowledge.backlink_count,
			knowledge.unsupported_summary_count,
			knowledge.untraced_section_count,
			knowledge.allowed_variance_count
		));
	}

	out.push('\n');
}
