use std::collections::HashSet;

use qdrant_client::qdrant::Filter;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	access::SharedSpaceGrantKey,
	docs::{DocType, types::trajectory::DocTrajectoryBuilder},
};

#[derive(Clone, Debug)]
pub(in crate::docs) struct DocsSearchL0Filters {
	pub(in crate::docs) scope: Option<String>,
	pub(in crate::docs) status: String,
	pub(in crate::docs) doc_type: Option<DocType>,
	pub(in crate::docs) sparse_mode: DocsSparseMode,
	pub(in crate::docs) domain: Option<String>,
	pub(in crate::docs) repo: Option<String>,
	pub(in crate::docs) agent_id: Option<String>,
	pub(in crate::docs) thread_id: Option<String>,
	pub(in crate::docs) updated_after: Option<OffsetDateTime>,
	pub(in crate::docs) updated_before: Option<OffsetDateTime>,
	pub(in crate::docs) ts_gte: Option<OffsetDateTime>,
	pub(in crate::docs) ts_lte: Option<OffsetDateTime>,
}

#[derive(Clone, Debug, FromRow)]
pub(in crate::docs) struct DocSearchRow {
	pub(in crate::docs) chunk_id: Uuid,
	pub(in crate::docs) doc_id: Uuid,
	pub(in crate::docs) scope: String,
	pub(in crate::docs) doc_type: String,
	pub(in crate::docs) project_id: String,
	pub(in crate::docs) agent_id: String,
	pub(in crate::docs) updated_at: OffsetDateTime,
	pub(in crate::docs) content_hash: String,
	pub(in crate::docs) chunk_hash: String,
	pub(in crate::docs) start_offset: i32,
	pub(in crate::docs) end_offset: i32,
	pub(in crate::docs) chunk_text: String,
}

pub(in crate::docs) struct DocsSearchL0Prepared {
	pub(in crate::docs) top_k: u32,
	pub(in crate::docs) candidate_k: u32,
	pub(in crate::docs) sparse_mode: DocsSparseMode,
	pub(in crate::docs) sparse_enabled: bool,
	pub(in crate::docs) now: OffsetDateTime,
	pub(in crate::docs) trajectory: DocTrajectoryBuilder,
	pub(in crate::docs) allowed_scopes: Vec<String>,
	pub(in crate::docs) shared_grants: HashSet<SharedSpaceGrantKey>,
	pub(in crate::docs) filter: Filter,
	pub(in crate::docs) vector: Vec<f32>,
	pub(in crate::docs) status: String,
}

#[derive(Debug)]
pub(in crate::docs) struct DocsSearchL0FiltersParsed {
	pub(in crate::docs) scope: Option<String>,
	pub(in crate::docs) status: String,
	pub(in crate::docs) doc_type: Option<DocType>,
	pub(in crate::docs) sparse_mode: DocsSparseMode,
	pub(in crate::docs) domain: Option<String>,
	pub(in crate::docs) repo: Option<String>,
	pub(in crate::docs) agent_id: Option<String>,
	pub(in crate::docs) thread_id: Option<String>,
}

#[derive(Debug)]
pub(in crate::docs) struct DocsSearchL0RangesParsed {
	pub(in crate::docs) updated_after: Option<OffsetDateTime>,
	pub(in crate::docs) updated_before: Option<OffsetDateTime>,
	pub(in crate::docs) ts_gte: Option<OffsetDateTime>,
	pub(in crate::docs) ts_lte: Option<OffsetDateTime>,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::docs) enum DocsSparseMode {
	Auto,
	On,
	Off,
}
impl DocsSparseMode {
	pub(in crate::docs) fn as_str(self) -> &'static str {
		match self {
			Self::Auto => "auto",
			Self::On => "on",
			Self::Off => "off",
		}
	}
}
