use crate::search::{
	self, ElfService, Result, SearchTrajectoryResponse, TraceGetRequest, TraceTrajectoryGetRequest,
};

impl ElfService {
	/// Loads full trajectory stages for one trace.
	pub async fn trace_trajectory_get(
		&self,
		req: TraceTrajectoryGetRequest,
	) -> Result<SearchTrajectoryResponse> {
		let base = self
			.trace_get(TraceGetRequest {
				tenant_id: req.tenant_id,
				project_id: req.project_id,
				agent_id: req.agent_id,
				trace_id: req.trace_id,
			})
			.await?;
		let stages = search::load_trace_trajectory_stages(&self.db.pool, req.trace_id).await?;
		let trajectory = search::build_trajectory_summary_from_stages(stages.as_slice());

		Ok(SearchTrajectoryResponse { trace: base.trace, trajectory, stages })
	}
}
