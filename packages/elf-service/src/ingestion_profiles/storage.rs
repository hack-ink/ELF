use serde_json::Value;
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
pub(super) struct ProfileRow {
	pub(super) profile_id: String,
	pub(super) version: i32,
	pub(super) profile: Value,
}

#[derive(FromRow)]
pub(super) struct ProfileMetadataRow {
	pub(super) profile_id: String,
	pub(super) version: i32,
	pub(super) profile: Value,
	pub(super) created_at: OffsetDateTime,
	pub(super) created_by: String,
}

#[derive(FromRow)]
pub(super) struct ProfileSummaryRow {
	pub(super) profile_id: String,
	pub(super) version: i32,
	pub(super) created_at: OffsetDateTime,
	pub(super) created_by: String,
}

#[derive(FromRow)]
pub(super) struct ProfileDefaultRow {
	pub(super) profile_id: String,
	pub(super) version: Option<i32>,
	pub(super) updated_at: OffsetDateTime,
}

pub(super) async fn next_profile_version(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	profile_id: &str,
) -> Result<i32> {
	let version = sqlx::query_scalar::<_, i32>(
		"SELECT COALESCE(MAX(version), 0) + 1 FROM memory_ingestion_profiles WHERE tenant_id=$1 AND project_id=$2 AND pipeline=$3 AND profile_id=$4",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(ADD_EVENT_PIPELINE)
	.bind(profile_id)
	.fetch_one(pool)
	.await?;

	Ok(version)
}

pub(super) async fn insert_profile_metadata(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	profile_id: &str,
	version: i32,
	profile: Value,
	created_by: &str,
) -> Result<Option<ProfileMetadataRow>> {
	let row = sqlx::query_as::<_, ProfileMetadataRow>(
		"\
INSERT INTO memory_ingestion_profiles (
	 tenant_id,
	 project_id,
	 pipeline,
	 profile_id,
	 version,
	 profile,
	 created_by
) VALUES ($1,$2,$3,$4,$5,$6::jsonb,$7)
ON CONFLICT DO NOTHING
RETURNING profile_id, version, profile, created_at, created_by",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(ADD_EVENT_PIPELINE)
	.bind(profile_id)
	.bind(version)
	.bind(profile)
	.bind(created_by)
	.fetch_optional(pool)
	.await?;

	Ok(row)
}

pub(super) async fn list_latest_profile_summaries(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
) -> Result<Vec<ProfileSummaryRow>> {
	let rows = sqlx::query_as::<_, ProfileSummaryRow>(
		"\
SELECT DISTINCT ON (profile_id)
	 profile_id, version, created_at, created_by
FROM memory_ingestion_profiles
WHERE tenant_id=$1 AND project_id=$2 AND pipeline=$3
ORDER BY profile_id, version DESC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(ADD_EVENT_PIPELINE)
	.fetch_all(pool)
	.await?;

	Ok(rows)
}

pub(super) async fn select_profile_metadata(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	selector: &IngestionProfileSelector,
) -> Result<ProfileMetadataRow> {
	let row = if let Some(version) = selector.version {
		sqlx::query_as::<_, ProfileMetadataRow>(
			"\
SELECT profile_id, version, profile, created_at, created_by
FROM memory_ingestion_profiles
WHERE tenant_id=$1 AND project_id=$2 AND pipeline=$3 AND profile_id=$4 AND version=$5",
		)
		.bind(tenant_id)
		.bind(project_id)
		.bind(ADD_EVENT_PIPELINE)
		.bind(selector.id.as_str())
		.bind(version)
		.fetch_optional(pool)
		.await?
	} else {
		sqlx::query_as::<_, ProfileMetadataRow>(
			"\
SELECT profile_id, version, profile, created_at, created_by
FROM memory_ingestion_profiles
WHERE tenant_id=$1 AND project_id=$2 AND pipeline=$3 AND profile_id=$4
ORDER BY version DESC
LIMIT 1",
		)
		.bind(tenant_id)
		.bind(project_id)
		.bind(ADD_EVENT_PIPELINE)
		.bind(selector.id.as_str())
		.fetch_optional(pool)
		.await?
	};

	row.ok_or_else(|| Error::InvalidRequest {
		message: format!(
			"Ingestion profile '{}' not found for tenant '{}' project '{}' pipeline '{}'.",
			selector.id, tenant_id, project_id, ADD_EVENT_PIPELINE,
		),
	})
}

pub(super) async fn list_profile_version_summaries(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	profile_id: &str,
) -> Result<Vec<ProfileSummaryRow>> {
	let rows = sqlx::query_as::<_, ProfileSummaryRow>(
		"\
SELECT profile_id, version, created_at, created_by
FROM memory_ingestion_profiles
WHERE tenant_id=$1 AND project_id=$2 AND pipeline=$3 AND profile_id=$4
ORDER BY version DESC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(ADD_EVENT_PIPELINE)
	.bind(profile_id)
	.fetch_all(pool)
	.await?;

	Ok(rows)
}

pub(super) async fn select_default_row(
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

pub(super) async fn upsert_default_row(
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

pub(super) async fn select_profile(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	selector: &IngestionProfileSelector,
) -> Result<ProfileRow> {
	let row = if let Some(version) = selector.version {
		sqlx::query_as::<_, ProfileRow>(
			"\
SELECT profile_id, version, profile
FROM memory_ingestion_profiles
WHERE tenant_id=$1 AND project_id=$2 AND pipeline=$3 AND profile_id=$4 AND version=$5",
		)
		.bind(tenant_id)
		.bind(project_id)
		.bind(ADD_EVENT_PIPELINE)
		.bind(selector.id.as_str())
		.bind(version)
		.fetch_optional(pool)
		.await?
	} else {
		sqlx::query_as::<_, ProfileRow>(
			"\
SELECT profile_id, version, profile
FROM memory_ingestion_profiles
WHERE tenant_id=$1 AND project_id=$2 AND pipeline=$3 AND profile_id=$4
ORDER BY version DESC
LIMIT 1",
		)
		.bind(tenant_id)
		.bind(project_id)
		.bind(ADD_EVENT_PIPELINE)
		.bind(selector.id.as_str())
		.fetch_optional(pool)
		.await?
	};

	row.ok_or_else(|| Error::InvalidRequest {
		message: format!(
			"Ingestion profile '{}' not found for tenant '{}' project '{}' pipeline '{}'.",
			selector.id, tenant_id, project_id, ADD_EVENT_PIPELINE
		),
	})
}

pub(super) async fn select_default_selector(
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

pub(super) async fn seed_default_profile(
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
