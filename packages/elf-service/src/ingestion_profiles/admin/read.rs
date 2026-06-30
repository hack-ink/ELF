use crate::{
	ElfService, Error, Result,
	ingestion_profiles::{
		storage,
		types::{
			AdminIngestionProfileGetRequest, AdminIngestionProfileListRequest,
			AdminIngestionProfileResponse, AdminIngestionProfileSummary,
			AdminIngestionProfilesListResponse, IngestionProfileSelector,
		},
	},
};

impl ElfService {
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
}
