use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

use crate::{
	Error, Result,
	ingestion_profiles::{
		ADD_EVENT_PIPELINE, DEFAULT_PROFILE_ID, DEFAULT_PROFILE_VERSION, profile,
		types::IngestionProfileSelector,
	},
};

#[derive(FromRow)]
pub(in crate::ingestion_profiles) struct ProfileDefaultRow {
	pub(in crate::ingestion_profiles) profile_id: String,
	pub(in crate::ingestion_profiles) version: Option<i32>,
	pub(in crate::ingestion_profiles) updated_at: OffsetDateTime,
}

pub(in crate::ingestion_profiles) async fn select_default_row(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
) -> Result<Option<ProfileDefaultRow>> {
	let row = sqlx::query_as::<_, ProfileDefaultRow>(
		"\
SELECT profile_id, version, updated_at
FROM memory_ingestion_profile_defaults
WHERE tenant_id=$1 AND project_id=$2 AND pipeline=$3",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(ADD_EVENT_PIPELINE)
	.fetch_optional(pool)
	.await?;

	Ok(row)
}

pub(in crate::ingestion_profiles) async fn upsert_default_row(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	profile_id: String,
	version: i32,
) -> Result<ProfileDefaultRow> {
	let row = sqlx::query_as::<_, ProfileDefaultRow>(
		"\
INSERT INTO memory_ingestion_profile_defaults (
	 tenant_id,
	 project_id,
	 pipeline,
	 profile_id,
	 version
) VALUES ($1,$2,$3,$4,$5)
ON CONFLICT (tenant_id, project_id, pipeline) DO UPDATE
SET profile_id = EXCLUDED.profile_id,
	version = EXCLUDED.version,
	updated_at = now()
RETURNING profile_id, version, updated_at",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(ADD_EVENT_PIPELINE)
	.bind(profile_id)
	.bind(version)
	.fetch_one(pool)
	.await?;

	Ok(row)
}

pub(in crate::ingestion_profiles) async fn select_default_selector(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
) -> Result<IngestionProfileSelector> {
	let row = sqlx::query_as::<_, (String, Option<i32>)>(
		"SELECT profile_id, version FROM memory_ingestion_profile_defaults WHERE tenant_id=$1 AND project_id=$2 AND pipeline=$3",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(ADD_EVENT_PIPELINE)
	.fetch_optional(pool)
	.await?;
	let row = match row {
		Some((profile_id, version)) => IngestionProfileSelector { id: profile_id, version },
		None => IngestionProfileSelector {
			id: DEFAULT_PROFILE_ID.to_string(),
			version: Some(DEFAULT_PROFILE_VERSION),
		},
	};

	Ok(row)
}

pub(in crate::ingestion_profiles) async fn seed_default_profile(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
) -> Result<()> {
	let profile =
		serde_json::to_value(profile::builtin_profile_v1()).map_err(|_| Error::InvalidRequest {
			message: "Failed to serialize default ingestion profile.".to_string(),
		})?;

	sqlx::query(
		"\
INSERT INTO memory_ingestion_profiles (
	tenant_id,
	project_id,
	pipeline,
	profile_id,
	version,
	profile
) VALUES ($1,$2,$3,$4,$5,$6::jsonb)
ON CONFLICT DO NOTHING",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(ADD_EVENT_PIPELINE)
	.bind(DEFAULT_PROFILE_ID)
	.bind(DEFAULT_PROFILE_VERSION)
	.bind(profile)
	.execute(pool)
	.await?;
	sqlx::query(
		"\
INSERT INTO memory_ingestion_profile_defaults (
	tenant_id,
	project_id,
	pipeline,
	profile_id,
	version
) VALUES ($1,$2,$3,$4,$5)
ON CONFLICT DO NOTHING",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(ADD_EVENT_PIPELINE)
	.bind(DEFAULT_PROFILE_ID)
	.bind(DEFAULT_PROFILE_VERSION)
	.execute(pool)
	.await?;

	Ok(())
}
