use crate::scoring::{
	ConsolidationJobReport, DimensionScoreReport, EvolutionJobReport, JobScoring, RealWorldJob,
	TypedStatus,
};

pub(in crate::scoring) fn score_declared_job(
	job: &RealWorldJob,
	status: TypedStatus,
	trap_ids_used: Vec<String>,
	evolution: Option<EvolutionJobReport>,
	consolidation: Option<ConsolidationJobReport>,
) -> JobScoring {
	JobScoring {
		status,
		normalized_score: 0.0,
		hard_fail_hits: Vec::new(),
		unsupported_claims: Vec::new(),
		wrong_result_count: 0,
		knowledge: None,
		trap_ids_used,
		dimension_scores: declared_not_encoded_dimension_scores(job),
		reason: job
			.encoding
			.reason
			.clone()
			.unwrap_or_else(|| "Job did not reach a runnable scoring state.".to_string()),
		evolution,
		consolidation,
		memory_summary: None,
		proactive_brief: None,
		scheduled_memory: None,
		work_continuity: None,
	}
}

fn declared_not_encoded_dimension_scores(job: &RealWorldJob) -> Vec<DimensionScoreReport> {
	job.scoring_rubric
		.dimensions
		.iter()
		.map(|(dimension_id, dimension)| DimensionScoreReport {
			dimension: dimension_id.clone(),
			score: 0.0,
			max_points: dimension.max_points,
			weight: dimension.weight,
		})
		.collect()
}
