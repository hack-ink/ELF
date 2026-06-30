use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(FromRow)]
pub(in crate::ingestion_profiles) struct ProfileRow {
	pub(in crate::ingestion_profiles) profile_id: String,
	pub(in crate::ingestion_profiles) version: i32,
	pub(in crate::ingestion_profiles) profile: Value,
}

#[derive(FromRow)]
pub(in crate::ingestion_profiles) struct ProfileMetadataRow {
	pub(in crate::ingestion_profiles) profile_id: String,
	pub(in crate::ingestion_profiles) version: i32,
	pub(in crate::ingestion_profiles) profile: Value,
	pub(in crate::ingestion_profiles) created_at: OffsetDateTime,
	pub(in crate::ingestion_profiles) created_by: String,
}

#[derive(FromRow)]
pub(in crate::ingestion_profiles) struct ProfileSummaryRow {
	pub(in crate::ingestion_profiles) profile_id: String,
	pub(in crate::ingestion_profiles) version: i32,
	pub(in crate::ingestion_profiles) created_at: OffsetDateTime,
	pub(in crate::ingestion_profiles) created_by: String,
}
