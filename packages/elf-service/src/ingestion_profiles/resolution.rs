use sqlx::PgPool;

use super::{
	profile::parse_profile,
	storage::{seed_default_profile, select_default_selector, select_profile},
	types::{IngestionProfileRef, IngestionProfileSelector, ResolvedIngestionProfile},
};
use crate::{Error, Result};

pub(crate) async fn resolve_add_event_profile(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	selector: Option<&IngestionProfileSelector>,
) -> Result<ResolvedIngestionProfile> {
	seed_default_profile(pool, tenant_id, project_id).await?;

	let selector = if let Some(selector) = selector {
		selector.clone()
	} else {
		select_default_selector(pool, tenant_id, project_id).await?
	};
	let row = select_profile(pool, tenant_id, project_id, &selector).await?;
	let parsed = parse_profile(row.profile)?;
	let merged = parsed.with_defaults();

	if merged.schema_version != 1 {
		return Err(Error::InvalidRequest {
			message: "Unsupported ingestion profile schema version.".to_string(),
		});
	}

	let prompt_schema = merged.prompt_schema.ok_or_else(|| Error::InvalidRequest {
		message: "Missing prompt schema in ingestion profile.".to_string(),
	})?;
	let prompt_system_template =
		merged.prompt_system_template.ok_or_else(|| Error::InvalidRequest {
			message: "Missing system prompt template in ingestion profile.".to_string(),
		})?;
	let prompt_user_template =
		merged.prompt_user_template.ok_or_else(|| Error::InvalidRequest {
			message: "Missing user prompt template in ingestion profile.".to_string(),
		})?;

	Ok(ResolvedIngestionProfile {
		profile_ref: IngestionProfileRef { id: row.profile_id, version: row.version },
		prompt_schema,
		prompt_system: prompt_system_template,
		prompt_user_template,
		model: merged.model,
		temperature: merged.temperature,
		timeout_ms: merged.timeout_ms,
	})
}
