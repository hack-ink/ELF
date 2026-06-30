use time::OffsetDateTime;

use crate::{
	ElfService, Error, PayloadLevel, Result,
	progressive_search::{
		details, storage,
		types::{
			SearchTimelineGroup, SearchTimelineRequest, SearchTimelineResponse,
			session::SearchSessionItemRecord,
		},
	},
};

impl ElfService {
	/// Reprojects a stored search session into timeline groups.
	pub async fn search_timeline(
		&self,
		req: SearchTimelineRequest,
	) -> Result<SearchTimelineResponse> {
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

		let expires_at = storage::touch_search_session(&self.db.pool, &session, now).await?;
		let payload_level = req.payload_level;
		let group_by = req.group_by.unwrap_or_else(|| {
			if payload_level == PayloadLevel::L0 { "none".to_string() } else { "day".to_string() }
		});

		match group_by.as_str() {
			"day" => details::build_timeline_by_day(
				session.search_session_id,
				expires_at,
				&session.items,
			),
			"none" => Ok(SearchTimelineResponse {
				search_session_id: session.search_session_id,
				expires_at,
				groups: vec![SearchTimelineGroup {
					date: "all".to_string(),
					items: session
						.items
						.iter()
						.map(SearchSessionItemRecord::to_index_item)
						.collect(),
				}],
			}),
			_ => Err(Error::InvalidRequest {
				message: "group_by must be one of: day, none.".to_string(),
			}),
		}
	}
}
