use crate::{
	ElfService, Error, Result, SearchRequest,
	progressive_search::types::{
		SearchIndexPlannedResponse, SearchIndexResponse, session::SearchSessionizePath,
	},
};

impl ElfService {
	/// Runs the default progressive-search path and returns indexed results.
	pub async fn search(&self, req: SearchRequest) -> Result<SearchIndexResponse> {
		let response = self.search_planned(req).await?;

		Ok(SearchIndexResponse {
			trace_id: response.trace_id,
			search_session_id: response.search_session_id,
			expires_at: response.expires_at,
			items: response.items,
			trajectory_summary: response.trajectory_summary,
		})
	}

	/// Runs quick-find search and stores a quick session without a query plan.
	pub async fn search_quick(&self, req: SearchRequest) -> Result<SearchIndexResponse> {
		self.search_sessionized(req, SearchSessionizePath::Quick).await.map(|output| output.index)
	}

	/// Runs planned search and stores a session with a query plan.
	pub async fn search_planned(&self, req: SearchRequest) -> Result<SearchIndexPlannedResponse> {
		let output = self.search_sessionized(req, SearchSessionizePath::Planned).await?;
		let query_plan = output.query_plan.ok_or_else(|| Error::Storage {
			message: "Planned search response is missing query_plan.".to_string(),
		})?;

		Ok(SearchIndexPlannedResponse {
			trace_id: output.index.trace_id,
			search_session_id: output.index.search_session_id,
			expires_at: output.index.expires_at,
			items: output.index.items,
			trajectory_summary: output.index.trajectory_summary,
			query_plan,
		})
	}
}
