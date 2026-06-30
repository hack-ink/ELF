use time::OffsetDateTime;

use crate::{
	ElfService, Error, Result,
	ingestion_profiles::{
		storage,
		types::{
			AdminIngestionProfileDefaultGetRequest, AdminIngestionProfileDefaultResponse,
			AdminIngestionProfileDefaultSetRequest, IngestionProfileSelector,
		},
	},
};

impl ElfService {
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
