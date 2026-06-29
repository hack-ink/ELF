use elf_config::LlmProviderConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

use crate::{Error, Result};

/// Selector for an ingestion profile and optional version.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IngestionProfileSelector {
	/// Profile identifier.
	pub id: String,
	/// Optional explicit version.
	pub version: Option<i32>,
}

/// Resolved ingestion-profile reference.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IngestionProfileRef {
	/// Profile identifier.
	pub id: String,
	/// Resolved version.
	pub version: i32,
}

/// Request payload for creating an ingestion profile version.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdminIngestionProfileCreateRequest {
	/// Tenant that owns the profile.
	pub tenant_id: String,
	/// Project that owns the profile.
	pub project_id: String,
	/// Profile identifier.
	pub profile_id: String,
	/// Optional explicit version number.
	pub version: Option<i32>,
	/// JSON profile payload.
	pub profile: Value,
	/// Actor creating the profile version.
	pub created_by: String,
}

/// Request payload for listing ingestion profiles.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdminIngestionProfileListRequest {
	/// Tenant that owns the profiles.
	pub tenant_id: String,
	/// Project that owns the profiles.
	pub project_id: String,
}

/// Request payload for fetching one ingestion profile.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdminIngestionProfileGetRequest {
	/// Tenant that owns the profile.
	pub tenant_id: String,
	/// Project that owns the profile.
	pub project_id: String,
	/// Profile identifier.
	pub profile_id: String,
	/// Optional explicit version.
	pub version: Option<i32>,
}

/// Request payload for listing all versions of one ingestion profile.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdminIngestionProfileVersionsListRequest {
	/// Tenant that owns the profile.
	pub tenant_id: String,
	/// Project that owns the profile.
	pub project_id: String,
	/// Profile identifier.
	pub profile_id: String,
}

/// Request payload for reading the default ingestion profile pointer.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdminIngestionProfileDefaultGetRequest {
	/// Tenant that owns the default pointer.
	pub tenant_id: String,
	/// Project that owns the default pointer.
	pub project_id: String,
}

/// Request payload for updating the default ingestion profile pointer.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdminIngestionProfileDefaultSetRequest {
	/// Tenant that owns the default pointer.
	pub tenant_id: String,
	/// Project that owns the default pointer.
	pub project_id: String,
	/// Profile identifier to make default.
	pub profile_id: String,
	/// Optional explicit version to make default.
	pub version: Option<i32>,
}

/// Response payload for one ingestion profile version.
#[derive(Clone, Debug, Serialize)]
pub struct AdminIngestionProfileResponse {
	/// Profile identifier.
	pub profile_id: String,
	/// Profile version.
	pub version: i32,
	/// JSON profile payload.
	pub profile: Value,
	#[serde(with = "crate::time_serde")]
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Actor that created the version.
	pub created_by: String,
}

/// Summary row for an ingestion profile version.
#[derive(Clone, Debug, Serialize)]
pub struct AdminIngestionProfileSummary {
	/// Profile identifier.
	pub profile_id: String,
	/// Profile version.
	pub version: i32,
	#[serde(with = "crate::time_serde")]
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Actor that created the version.
	pub created_by: String,
}

/// Response payload for listing ingestion profiles.
#[derive(Clone, Debug, Serialize)]
pub struct AdminIngestionProfilesListResponse {
	/// Returned profile summaries.
	pub profiles: Vec<AdminIngestionProfileSummary>,
}

/// Response payload for listing versions of one ingestion profile.
#[derive(Clone, Debug, Serialize)]
pub struct AdminIngestionProfileVersionsListResponse {
	/// Returned profile-version summaries.
	pub profiles: Vec<AdminIngestionProfileSummary>,
}

/// Response payload for reading the default ingestion profile pointer.
#[derive(Clone, Debug, Serialize)]
pub struct AdminIngestionProfileDefaultResponse {
	/// Default profile identifier.
	pub profile_id: String,
	/// Default profile version, when pinned.
	pub version: Option<i32>,
	#[serde(with = "crate::time_serde")]
	/// Last update timestamp for the default pointer.
	pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedIngestionProfile {
	pub profile_ref: IngestionProfileRef,
	pub prompt_schema: Value,
	pub prompt_system: String,
	pub prompt_user_template: String,
	pub model: Option<String>,
	pub temperature: Option<f32>,
	pub timeout_ms: Option<u64>,
}
impl ResolvedIngestionProfile {
	pub(crate) fn build_extractor_messages(
		&self,
		messages_json: &str,
		max_notes: u32,
		max_note_chars: u32,
	) -> Result<Vec<Value>> {
		let schema =
			serde_json::to_string(&self.prompt_schema).map_err(|_| Error::InvalidRequest {
				message: "Failed to serialize ingestion profile schema.".to_string(),
			})?;
		let user_prompt = self
			.prompt_user_template
			.replace("{SCHEMA}", &schema)
			.replace("{MAX_NOTES}", max_notes.to_string().as_str())
			.replace("{MAX_NOTE_CHARS}", max_note_chars.to_string().as_str())
			.replace("{MESSAGES_JSON}", messages_json);

		Ok(vec![
			serde_json::json!({ "role": "system", "content": self.prompt_system.clone() }),
			serde_json::json!({ "role": "user", "content": user_prompt }),
		])
	}

	pub(crate) fn resolved_llm_config(&self, base: &LlmProviderConfig) -> LlmProviderConfig {
		LlmProviderConfig {
			provider_id: base.provider_id.clone(),
			api_base: base.api_base.clone(),
			api_key: base.api_key.clone(),
			path: base.path.clone(),
			model: self.model.clone().unwrap_or_else(|| base.model.clone()),
			temperature: self.temperature.unwrap_or(base.temperature),
			timeout_ms: self.timeout_ms.unwrap_or(base.timeout_ms),
			default_headers: base.default_headers.clone(),
		}
	}
}
