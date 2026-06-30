use crate::{
	ElfService, Error, Result,
	ingestion_profiles::{
		storage,
		types::{
			AdminIngestionProfileSummary, AdminIngestionProfileVersionsListRequest,
			AdminIngestionProfileVersionsListResponse,
		},
	},
};

impl ElfService {
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
}
