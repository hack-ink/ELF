use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	QueryPlan, SearchTrajectorySummary,
	progressive_search::types::{SearchIndexItem, SearchIndexResponse, SearchSessionMode},
};

pub(in crate::progressive_search) const SESSION_SLIDING_TTL_HOURS: i64 = 6;
pub(in crate::progressive_search) const SESSION_ABSOLUTE_TTL_HOURS: i64 = 24;

pub(in crate::progressive_search) struct HitItem {
	pub(in crate::progressive_search) note_id: Uuid,
	pub(in crate::progressive_search) chunk_id: Uuid,
	pub(in crate::progressive_search) rank: u32,
	pub(in crate::progressive_search) final_score: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::progressive_search) enum SearchSessionizePath {
	Quick,
	Planned,
}

pub(in crate::progressive_search) struct SearchSessionizedOutput {
	pub(in crate::progressive_search) index: SearchIndexResponse,
	pub(in crate::progressive_search) query_plan: Option<QueryPlan>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(in crate::progressive_search) struct SearchSessionItemRecord {
	pub(in crate::progressive_search) rank: u32,
	pub(in crate::progressive_search) note_id: Uuid,
	pub(in crate::progressive_search) chunk_id: Uuid,
	pub(in crate::progressive_search) final_score: f32,
	#[serde(with = "crate::time_serde")]
	pub(in crate::progressive_search) updated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	pub(in crate::progressive_search) expires_at: Option<OffsetDateTime>,
	pub(in crate::progressive_search) r#type: String,
	pub(in crate::progressive_search) key: Option<String>,
	pub(in crate::progressive_search) scope: String,
	pub(in crate::progressive_search) importance: f32,
	pub(in crate::progressive_search) confidence: f32,
	pub(in crate::progressive_search) summary: String,
}
impl SearchSessionItemRecord {
	pub(in crate::progressive_search) fn to_index_item(&self) -> SearchIndexItem {
		SearchIndexItem {
			note_id: self.note_id,
			r#type: self.r#type.clone(),
			key: self.key.clone(),
			scope: self.scope.clone(),
			importance: self.importance,
			confidence: self.confidence,
			updated_at: self.updated_at,
			expires_at: self.expires_at,
			final_score: self.final_score,
			summary: self.summary.clone(),
		}
	}
}

pub(in crate::progressive_search) struct SearchSession {
	pub(in crate::progressive_search) search_session_id: Uuid,
	pub(in crate::progressive_search) trace_id: Uuid,
	pub(in crate::progressive_search) tenant_id: String,
	pub(in crate::progressive_search) project_id: String,
	pub(in crate::progressive_search) agent_id: String,
	pub(in crate::progressive_search) read_profile: String,
	pub(in crate::progressive_search) query: String,
	pub(in crate::progressive_search) mode: SearchSessionMode,
	pub(in crate::progressive_search) trajectory_summary: Option<SearchTrajectorySummary>,
	pub(in crate::progressive_search) query_plan: Option<QueryPlan>,
	pub(in crate::progressive_search) items: Vec<SearchSessionItemRecord>,
	pub(in crate::progressive_search) created_at: OffsetDateTime,
	pub(in crate::progressive_search) expires_at: OffsetDateTime,
}

#[derive(FromRow)]
pub(in crate::progressive_search) struct SearchSessionRow {
	pub(in crate::progressive_search) search_session_id: Uuid,
	pub(in crate::progressive_search) trace_id: Uuid,
	pub(in crate::progressive_search) tenant_id: String,
	pub(in crate::progressive_search) project_id: String,
	pub(in crate::progressive_search) agent_id: String,
	pub(in crate::progressive_search) read_profile: String,
	pub(in crate::progressive_search) query: String,
	pub(in crate::progressive_search) mode: String,
	pub(in crate::progressive_search) trajectory_summary: Option<Value>,
	pub(in crate::progressive_search) query_plan: Option<Value>,
	pub(in crate::progressive_search) items: Value,
	pub(in crate::progressive_search) created_at: OffsetDateTime,
	pub(in crate::progressive_search) expires_at: OffsetDateTime,
}

pub(in crate::progressive_search) struct NewSearchSession<'a> {
	pub(in crate::progressive_search) search_session_id: Uuid,
	pub(in crate::progressive_search) trace_id: Uuid,
	pub(in crate::progressive_search) tenant_id: &'a str,
	pub(in crate::progressive_search) project_id: &'a str,
	pub(in crate::progressive_search) agent_id: &'a str,
	pub(in crate::progressive_search) read_profile: &'a str,
	pub(in crate::progressive_search) query: &'a str,
	pub(in crate::progressive_search) mode: SearchSessionMode,
	pub(in crate::progressive_search) trajectory_summary: Option<&'a SearchTrajectorySummary>,
	pub(in crate::progressive_search) query_plan: Option<&'a QueryPlan>,
	pub(in crate::progressive_search) items: &'a [SearchSessionItemRecord],
	pub(in crate::progressive_search) created_at: OffsetDateTime,
	pub(in crate::progressive_search) expires_at: OffsetDateTime,
}
