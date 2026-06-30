use serde_json::Value;
use sqlx::PgPool;

use crate::{
	Error, Result,
	ingestion_profiles::{
		ADD_EVENT_PIPELINE,
		storage::{ProfileMetadataRow, ProfileRow, ProfileSummaryRow},
		types::IngestionProfileSelector,
	},
};

pub(in crate::ingestion_profiles) async fn next_profile_version(
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

pub(in crate::ingestion_profiles) async fn insert_profile_metadata(
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

pub(in crate::ingestion_profiles) async fn list_latest_profile_summaries(
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

pub(in crate::ingestion_profiles) async fn select_profile_metadata(
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

pub(in crate::ingestion_profiles) async fn list_profile_version_summaries(
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

pub(in crate::ingestion_profiles) async fn select_profile(
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
