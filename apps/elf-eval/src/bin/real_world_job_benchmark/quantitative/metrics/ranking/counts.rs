use crate::{BTreeSet, RealWorldJob, quantitative::metrics::ranking::queries};

pub(in crate::quantitative) fn ranking_query_ids(source_jobs: &[RealWorldJob]) -> BTreeSet<&str> {
	source_jobs
		.iter()
		.filter(|job| queries::is_ranking_query(job))
		.map(|job| job.job_id.as_str())
		.collect()
}

pub(in crate::quantitative) fn ranking_query_count(source_jobs: &[RealWorldJob]) -> usize {
	ranking_query_ids(source_jobs).len()
}

pub(in crate::quantitative) fn explicit_qrel_query_count(source_jobs: &[RealWorldJob]) -> usize {
	source_jobs.iter().filter(|job| !job.expected_answer.relevance_judgments.is_empty()).count()
}
