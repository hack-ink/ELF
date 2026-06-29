use time::OffsetDateTime;

use crate::{
	ElfService, Error, Result,
	ingestion_profiles::{
		ADD_EVENT_PIPELINE, profile,
		storage::{self},
		types::{
			AdminIngestionProfileCreateRequest, AdminIngestionProfileDefaultGetRequest,
			AdminIngestionProfileDefaultResponse, AdminIngestionProfileDefaultSetRequest,
			AdminIngestionProfileGetRequest, AdminIngestionProfileListRequest,
			AdminIngestionProfileResponse, AdminIngestionProfileSummary,
			AdminIngestionProfileVersionsListRequest, AdminIngestionProfileVersionsListResponse,
			AdminIngestionProfilesListResponse, IngestionProfileSelector,
		},
	},
};

impl ElfService {
	/// Creates a new ingestion profile version.
	pub async fn admin_ingestion_profile_create(
		&self,
		req: AdminIngestionProfileCreateRequest,
	) -> Result<AdminIngestionProfileResponse> {
		let profile_id = req.profile_id.trim().to_string();
		let created_by = req.created_by.trim().to_string();

		if profile_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "profile_id must be non-empty.".to_string(),
			});
		}
		if created_by.is_empty() {
			return Err(Error::InvalidRequest {
				message: "created_by must be non-empty.".to_string(),
			});
		}
		if !req.profile.is_object() {
			return Err(Error::InvalidRequest {
				message: "profile must be a JSON object.".to_string(),
			});
		}

		let _ = profile::parse_profile(req.profile.clone())?;
		let version = match req.version {
			Some(version) if version > 0 => version,
			Some(_) => {
				return Err(Error::InvalidRequest {
					message: "version must be greater than 0.".to_string(),
				});
			},
			None =>
				storage::next_profile_version(
					&self.db.pool,
					req.tenant_id.as_str(),
					req.project_id.as_str(),
					profile_id.as_str(),
				)
				.await?,
		};
		let row = storage::insert_profile_metadata(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			profile_id.as_str(),
			version,
			req.profile,
			created_by.as_str(),
		)
		.await?;
		let row = row.ok_or_else(|| Error::Conflict {
			message: format!(
				"Ingestion profile '{}' version {} already exists for tenant '{}' project '{}' pipeline '{}'.",
				profile_id, version, req.tenant_id, req.project_id, ADD_EVENT_PIPELINE,
			),
		})?;

		Ok(AdminIngestionProfileResponse {
			profile_id: row.profile_id,
			version: row.version,
			profile: row.profile,
			created_at: row.created_at,
			created_by: row.created_by,
		})
	}

	/// Lists the latest visible ingestion profile versions.
	pub async fn admin_ingestion_profiles_list(
		&self,
		req: AdminIngestionProfileListRequest,
	) -> Result<AdminIngestionProfilesListResponse> {
		let rows = storage::list_latest_profile_summaries(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
		)
		.await?;
		let profiles = rows
			.into_iter()
			.map(|row| AdminIngestionProfileSummary {
				profile_id: row.profile_id,
				version: row.version,
				created_at: row.created_at,
				created_by: row.created_by,
			})
			.collect();

		Ok(AdminIngestionProfilesListResponse { profiles })
	}

	/// Fetches one ingestion profile version.
	pub async fn admin_ingestion_profile_get(
		&self,
		req: AdminIngestionProfileGetRequest,
	) -> Result<AdminIngestionProfileResponse> {
		let selector = IngestionProfileSelector {
			id: req.profile_id.trim().to_string(),
			version: req.version,
		};

		if selector.id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "profile_id must be non-empty.".to_string(),
			});
		}

		if let Some(version) = selector.version
			&& version <= 0
		{
			return Err(Error::InvalidRequest {
				message: "version must be greater than 0.".to_string(),
			});
		}

		let row = storage::select_profile_metadata(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			&selector,
		)
		.await?;

		Ok(AdminIngestionProfileResponse {
			profile_id: row.profile_id,
			version: row.version,
			profile: row.profile,
			created_at: row.created_at,
			created_by: row.created_by,
		})
	}

	/// Lists all versions for one ingestion profile.
	pub async fn admin_ingestion_profile_versions_list(
		&self,
		req: AdminIngestionProfileVersionsListRequest,
	) -> Result<AdminIngestionProfileVersionsListResponse> {
		let profile_id = req.profile_id.trim().to_string();

		if profile_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "profile_id must be non-empty.".to_string(),
			});
		}

		let rows = storage::list_profile_version_summaries(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			profile_id.as_str(),
		)
		.await?;
		let profiles = rows
			.into_iter()
			.map(|row| AdminIngestionProfileSummary {
				profile_id: row.profile_id,
				version: row.version,
				created_at: row.created_at,
				created_by: row.created_by,
			})
			.collect();

		Ok(AdminIngestionProfileVersionsListResponse { profiles })
	}

	/// Reads the default ingestion profile pointer.
	pub async fn admin_ingestion_profile_default_get(
		&self,
		req: AdminIngestionProfileDefaultGetRequest,
	) -> Result<AdminIngestionProfileDefaultResponse> {
		storage::seed_default_profile(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
		)
		.await?;

		let row = storage::select_default_row(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
		)
		.await?;
		let row = match row {
			Some(row) => row,
			None => {
				let selector = storage::select_default_selector(
					&self.db.pool,
					req.tenant_id.as_str(),
					req.project_id.as_str(),
				)
				.await?;

				return Ok(AdminIngestionProfileDefaultResponse {
					profile_id: selector.id,
					version: selector.version,
					updated_at: OffsetDateTime::now_utc(),
				});
			},
		};

		Ok(AdminIngestionProfileDefaultResponse {
			profile_id: row.profile_id,
			version: row.version,
			updated_at: row.updated_at,
		})
	}

	/// Updates the default ingestion profile pointer.
	pub async fn admin_ingestion_profile_default_set(
		&self,
		req: AdminIngestionProfileDefaultSetRequest,
	) -> Result<AdminIngestionProfileDefaultResponse> {
		let profile_id = req.profile_id.trim().to_string();

		if profile_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "profile_id must be non-empty.".to_string(),
			});
		}

		if let Some(version) = req.version
			&& version <= 0
		{
			return Err(Error::InvalidRequest {
				message: "version must be greater than 0.".to_string(),
			});
		}

		let selector = IngestionProfileSelector { id: profile_id.clone(), version: req.version };
		let row = storage::select_profile_metadata(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			&selector,
		)
		.await?;
		let version = row.version;
		let row = storage::upsert_default_row(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			row.profile_id,
			version,
		)
		.await?;

		Ok(AdminIngestionProfileDefaultResponse {
			profile_id: row.profile_id,
			version: row.version,
			updated_at: row.updated_at,
		})
	}
}
