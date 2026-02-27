use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool};

use elf_config::LlmProviderConfig;

use crate::{ElfService, Error, Result};
use time::OffsetDateTime;

const ADD_EVENT_PIPELINE: &str = "add_event";
const DEFAULT_PROFILE_ID: &str = "default";
const DEFAULT_PROFILE_VERSION: i32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IngestionProfileSelector {
	pub id: String,
	pub version: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IngestionProfileRef {
	pub id: String,
	pub version: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdminIngestionProfileCreateRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub profile_id: String,
	pub version: Option<i32>,
	pub profile: Value,
	pub created_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdminIngestionProfileListRequest {
	pub tenant_id: String,
	pub project_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdminIngestionProfileGetRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub profile_id: String,
	pub version: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdminIngestionProfileVersionsListRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub profile_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdminIngestionProfileDefaultGetRequest {
	pub tenant_id: String,
	pub project_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdminIngestionProfileDefaultSetRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub profile_id: String,
	pub version: Option<i32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminIngestionProfileResponse {
	pub profile_id: String,
	pub version: i32,
	pub profile: Value,
	#[serde(with = "crate::time_serde")]
	pub created_at: OffsetDateTime,
	pub created_by: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminIngestionProfileSummary {
	pub profile_id: String,
	pub version: i32,
	#[serde(with = "crate::time_serde")]
	pub created_at: OffsetDateTime,
	pub created_by: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminIngestionProfilesListResponse {
	pub profiles: Vec<AdminIngestionProfileSummary>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminIngestionProfileVersionsListResponse {
	pub profiles: Vec<AdminIngestionProfileSummary>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminIngestionProfileDefaultResponse {
	pub profile_id: String,
	pub version: Option<i32>,
	#[serde(with = "crate::time_serde")]
	pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct IngestionProfileV1 {
	#[serde(default = "default_schema_version")]
	schema_version: i32,
	#[serde(default)]
	prompt_schema: Option<Value>,
	#[serde(default)]
	prompt_system_template: Option<String>,
	#[serde(default)]
	prompt_user_template: Option<String>,
	#[serde(default)]
	model: Option<String>,
	#[serde(default)]
	temperature: Option<f32>,
	#[serde(default)]
	timeout_ms: Option<u64>,
}

fn default_schema_version() -> i32 {
	1
}

impl IngestionProfileV1 {
	fn with_defaults(self) -> Self {
		let defaults = builtin_profile_v1();

		let mut merged = defaults;

		if self.schema_version != 0 {
			merged.schema_version = self.schema_version;
		}
		merged.prompt_schema = self.prompt_schema.or(merged.prompt_schema);
		merged.prompt_system_template =
			self.prompt_system_template.or(merged.prompt_system_template);
		merged.prompt_user_template = self.prompt_user_template.or(merged.prompt_user_template);
		merged.model = self.model.or(merged.model);
		merged.temperature = self.temperature.or(merged.temperature);
		merged.timeout_ms = self.timeout_ms.or(merged.timeout_ms);

		merged
	}
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

#[derive(FromRow)]
struct ProfileRow {
	profile_id: String,
	version: i32,
	profile: Value,
}

#[derive(FromRow)]
struct ProfileMetadataRow {
	profile_id: String,
	version: i32,
	profile: Value,
	created_at: OffsetDateTime,
	created_by: String,
}

#[derive(FromRow)]
struct ProfileSummaryRow {
	profile_id: String,
	version: i32,
	created_at: OffsetDateTime,
	created_by: String,
}

#[derive(FromRow)]
struct ProfileDefaultRow {
	profile_id: String,
	version: Option<i32>,
	updated_at: OffsetDateTime,
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

impl ElfService {
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

		let _ = parse_profile(req.profile.clone())?;
		let version = match req.version {
			Some(version) if version > 0 => version,
			Some(_) => {
				return Err(Error::InvalidRequest {
					message: "version must be greater than 0.".to_string(),
				});
			},
			None => {
				sqlx::query_scalar::<_, i32>(
					"SELECT COALESCE(MAX(version), 0) + 1 FROM memory_ingestion_profiles WHERE tenant_id=$1 AND project_id=$2 AND pipeline=$3 AND profile_id=$4",
				)
				.bind(req.tenant_id.as_str())
				.bind(req.project_id.as_str())
				.bind(ADD_EVENT_PIPELINE)
				.bind(profile_id.as_str())
				.fetch_one(&self.db.pool)
				.await?
			}
		};

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
		.bind(req.tenant_id.as_str())
		.bind(req.project_id.as_str())
		.bind(ADD_EVENT_PIPELINE)
		.bind(profile_id.as_str())
		.bind(version)
		.bind(req.profile)
		.bind(created_by.as_str())
		.fetch_optional(&self.db.pool)
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

	pub async fn admin_ingestion_profiles_list(
		&self,
		req: AdminIngestionProfileListRequest,
	) -> Result<AdminIngestionProfilesListResponse> {
		let rows = sqlx::query_as::<_, ProfileSummaryRow>(
			"\
SELECT DISTINCT ON (profile_id)
 	 profile_id, version, created_at, created_by
FROM memory_ingestion_profiles
WHERE tenant_id=$1 AND project_id=$2 AND pipeline=$3
ORDER BY profile_id, version DESC",
		)
		.bind(req.tenant_id.as_str())
		.bind(req.project_id.as_str())
		.bind(ADD_EVENT_PIPELINE)
		.fetch_all(&self.db.pool)
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

		let row = select_profile_metadata(
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

		let rows = sqlx::query_as::<_, ProfileSummaryRow>(
			"\
SELECT profile_id, version, created_at, created_by
FROM memory_ingestion_profiles
WHERE tenant_id=$1 AND project_id=$2 AND pipeline=$3 AND profile_id=$4
ORDER BY version DESC",
		)
		.bind(req.tenant_id.as_str())
		.bind(req.project_id.as_str())
		.bind(ADD_EVENT_PIPELINE)
		.bind(profile_id)
		.fetch_all(&self.db.pool)
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

	pub async fn admin_ingestion_profile_default_get(
		&self,
		req: AdminIngestionProfileDefaultGetRequest,
	) -> Result<AdminIngestionProfileDefaultResponse> {
		seed_default_profile(&self.db.pool, req.tenant_id.as_str(), req.project_id.as_str())
			.await?;

		let row = sqlx::query_as::<_, ProfileDefaultRow>(
			"\
SELECT profile_id, version, updated_at
FROM memory_ingestion_profile_defaults
WHERE tenant_id=$1 AND project_id=$2 AND pipeline=$3",
		)
		.bind(req.tenant_id.as_str())
		.bind(req.project_id.as_str())
		.bind(ADD_EVENT_PIPELINE)
		.fetch_optional(&self.db.pool)
		.await?;

		let row = match row {
			Some(row) => row,
			None => {
				let selector = select_default_selector(
					&self.db.pool,
					req.tenant_id.as_str(),
					req.project_id.as_str(),
				)
				.await?;

				ProfileDefaultRow {
					profile_id: selector.id,
					version: selector.version,
					updated_at: OffsetDateTime::now_utc(),
				}
			},
		};

		Ok(AdminIngestionProfileDefaultResponse {
			profile_id: row.profile_id,
			version: row.version,
			updated_at: row.updated_at,
		})
	}

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

		let row = select_profile_metadata(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			&selector,
		)
		.await?;
		let version = row.version;

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
		.bind(req.tenant_id.as_str())
		.bind(req.project_id.as_str())
		.bind(ADD_EVENT_PIPELINE)
		.bind(row.profile_id)
		.bind(version)
		.fetch_one(&self.db.pool)
		.await?;

		Ok(AdminIngestionProfileDefaultResponse {
			profile_id: row.profile_id,
			version: row.version,
			updated_at: row.updated_at,
		})
	}
}

async fn select_profile_metadata(
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

async fn select_profile(
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

async fn select_default_selector(
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

async fn seed_default_profile(pool: &PgPool, tenant_id: &str, project_id: &str) -> Result<()> {
	let profile =
		serde_json::to_value(builtin_profile_v1()).map_err(|_| Error::InvalidRequest {
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

fn parse_profile(profile: Value) -> Result<IngestionProfileV1> {
	let parsed = serde_json::from_value::<IngestionProfileV1>(profile.clone()).or_else(|_| {
		if profile.is_object() {
			Ok(IngestionProfileV1 {
				schema_version: 1,
				prompt_schema: Some(profile),
				prompt_system_template: None,
				prompt_user_template: None,
				model: None,
				temperature: None,
				timeout_ms: None,
			})
		} else {
			Err(Error::InvalidRequest {
				message: "Ingestion profile JSON has unsupported format.".to_string(),
			})
		}
	})?;

	Ok(parsed)
}

fn builtin_profile_v1() -> IngestionProfileV1 {
	IngestionProfileV1 {
		schema_version: 1,
		prompt_schema: Some(builtin_profile_schema()),
		prompt_system_template: Some(
			"You are a memory extraction engine for an agent memory system. Output must be valid JSON only and must match the provided schema exactly. \
Extract at most MAX_NOTES high-signal, cross-session reusable memory notes from the given messages. \
Each note must be one English sentence and must not contain any non-English text. \
The structured field is optional. If present, summary must be short, facts must be short sentences supported by the evidence quotes, and concepts must be short phrases. \
structured.entities and structured.relations should mirror the structured schema with optional entity and relation metadata and relation timestamps. \
Preserve numbers, dates, percentages, currency amounts, tickers, URLs, and code snippets exactly. \
Never store secrets or PII: API keys, tokens, private keys, seed phrases, passwords, bank IDs, personal addresses. \
For every note, provide 1 to 2 evidence quotes copied verbatim from the input messages and include the message_index. \
If you cannot provide verbatim evidence, omit the note. \
If content is ephemeral or not useful long-term, return an empty notes array."
				.to_string(),
		),
		prompt_user_template: Some(
			"Return JSON matching this exact schema:\n{SCHEMA}\nConstraints:\n- MAX_NOTES = {MAX_NOTES}\n- MAX_NOTE_CHARS = {MAX_NOTE_CHARS}\nHere are the messages as JSON:\n{MESSAGES_JSON}"
				.to_string(),
		),
		model: None,
		temperature: None,
		timeout_ms: None,
	}
}

fn builtin_profile_schema() -> Value {
	serde_json::json!({
		"notes": [
			{
				"type": "preference|constraint|decision|profile|fact|plan",
				"key": "string|null",
				"text": "English-only sentence <= MAX_NOTE_CHARS",
				"structured": {
					"summary": "string|null",
					"facts": "string[]|null",
					"concepts": "string[]|null",
					"entities": [
						{
							"canonical": "string|null",
							"kind": "string|null",
							"aliases": "string[]|null"
						}
					],
					"relations": [
						{
							"subject": {
								"canonical": "string|null",
								"kind": "string|null",
								"aliases": "string[]|null"
							},
							"predicate": "string",
							"object": {
								"entity": {
									"canonical": "string|null",
									"kind": "string|null",
									"aliases": "string[]|null"
								},
								"value": "string|null"
							},
							"valid_from": "string|null",
							"valid_to": "string|null"
						}
					]
				},
				"importance": 0.0,
				"confidence": 0.0,
				"ttl_days": "number|null",
				"scope_suggestion": "agent_private|project_shared|org_shared|null",
				"evidence": [
					{ "message_index": "number", "quote": "string" }
				],
				"reason": "string"
			}
		]
	})
}
