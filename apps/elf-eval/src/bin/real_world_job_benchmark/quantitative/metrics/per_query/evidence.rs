use crate::{BTreeMap, JobReport, RealWorldJob};

pub(super) fn relevance_grades(
	source_job: &RealWorldJob,
	job: &JobReport,
) -> BTreeMap<String, f64> {
	let explicit = source_job
		.expected_answer
		.relevance_judgments
		.iter()
		.map(|judgment| (judgment.evidence_id.clone(), judgment.grade))
		.collect::<BTreeMap<_, _>>();

	if !explicit.is_empty() {
		return explicit;
	}

	job.expected_evidence.iter().map(|evidence| (evidence.evidence_id.clone(), 1.0)).collect()
}

pub(super) fn qrel_source(source_job: &RealWorldJob, empty: bool) -> &'static str {
	if !source_job.expected_answer.relevance_judgments.is_empty() {
		"explicit_qrels"
	} else if empty {
		"not_encoded"
	} else {
		"expected_evidence_fallback"
	}
}
