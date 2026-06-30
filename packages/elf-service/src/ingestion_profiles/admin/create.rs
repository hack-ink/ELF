use crate::{
	ElfService, Error, Result,
	ingestion_profiles::{
		ADD_EVENT_PIPELINE, profile, storage,
		types::{AdminIngestionProfileCreateRequest, AdminIngestionProfileResponse},
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
}
