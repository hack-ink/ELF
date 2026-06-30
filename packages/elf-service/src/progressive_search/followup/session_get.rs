use time::OffsetDateTime;

use crate::{
	ElfService, Error, Result,
	progressive_search::{
		details, storage,
		types::{SearchIndexItem, SearchSessionGetRequest, SearchSessionGetResponse},
	},
};

impl ElfService {
	/// Reloads a stored search session and optionally extends its TTL.
	pub async fn search_session_get(
		&self,
		req: SearchSessionGetRequest,
	) -> Result<SearchSessionGetResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let now = OffsetDateTime::now_utc();
		let session =
			storage::load_search_session(&self.db.pool, req.search_session_id, now).await?;

		details::validate_search_session_access(&session, tenant_id, project_id, agent_id)?;

		let touch = req.touch.unwrap_or(true);
		let expires_at = if touch {
			storage::touch_search_session(&self.db.pool, &session, now).await?
		} else {
			session.expires_at
		};
		let top_k = req.top_k.unwrap_or(self.cfg.memory.top_k).max(1);
		let items: Vec<SearchIndexItem> = session
			.items
			.into_iter()
			.take(top_k as usize)
			.map(|item| item.to_index_item())
			.collect();

		Ok(SearchSessionGetResponse {
			trace_id: session.trace_id,
			search_session_id: session.search_session_id,
			expires_at,
			items,
			mode: session.mode,
			query_plan: session.query_plan,
			trajectory_summary: session.trajectory_summary,
		})
	}
}
