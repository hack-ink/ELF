use crate::search::{
	ElfService, RawSearchPath, Result, SearchRawPlannedResponse, SearchRequest, SearchResponse,
};

impl ElfService {
	/// Runs the quick raw-search path and returns ranked items without a query plan.
	pub async fn search_raw_quick(&self, req: SearchRequest) -> Result<SearchResponse> {
		self.execute_search_raw_path(req, RawSearchPath::Quick).await.map(|response| {
			SearchResponse {
				trace_id: response.trace_id,
				items: response.items,
				trajectory_summary: response.trajectory_summary,
			}
		})
	}

	/// Runs the planned raw-search path and returns ranked items plus a query plan.
	pub async fn search_raw_planned(&self, req: SearchRequest) -> Result<SearchRawPlannedResponse> {
		self.execute_search_raw_path(req, RawSearchPath::Planned).await
	}

	/// Runs the default raw-search path and returns ranked items.
	pub async fn search_raw(&self, req: SearchRequest) -> Result<SearchResponse> {
		self.search_raw_planned(req).await.map(|response| SearchResponse {
			trace_id: response.trace_id,
			items: response.items,
			trajectory_summary: response.trajectory_summary,
		})
	}
}
