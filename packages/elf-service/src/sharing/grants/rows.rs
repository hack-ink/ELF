use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(FromRow)]
pub(in crate::sharing) struct SpaceGrantRow {
	pub(in crate::sharing) scope: String,
	pub(in crate::sharing) grantee_kind: String,
	pub(in crate::sharing) grantee_agent_id: Option<String>,
	pub(in crate::sharing) granted_by_agent_id: String,
	pub(in crate::sharing) granted_at: OffsetDateTime,
}
