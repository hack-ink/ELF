use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, QueryBuilder};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ElfService, Error, Result, access};
use elf_storage::models::MemoryNote;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: Option<String>,
	pub scope: Option<String>,
	pub status: Option<String>,
	pub r#type: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListItem {
	pub note_id: Uuid,
	pub r#type: String,
	pub key: Option<String>,
	pub scope: String,
	pub status: String,
	pub text: String,
	pub importance: f32,
	pub confidence: f32,
	#[serde(with = "crate::time_serde")]
	pub updated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	pub expires_at: Option<OffsetDateTime>,
	pub source_ref: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListResponse {
	pub items: Vec<ListItem>,
}

impl ElfService {
	pub async fn list(&self, req: ListRequest) -> Result<ListResponse> {
		let now = OffsetDateTime::now_utc();
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.as_ref().map(|value| value.trim()).unwrap_or("");
		let requested_status = requested_list_status(req.status.as_ref());
		let status_for_note_read =
			requested_status.unwrap_or("active").eq_ignore_ascii_case("active");
		let non_private_scopes = list_non_private_scopes(req.scope.as_ref());

		validate_list_request(&req, tenant_id, project_id, agent_id, &self.cfg.scopes.allowed)?;

		let shared_grants =
			list_shared_grants(&self.db.pool, tenant_id, project_id, agent_id, &non_private_scopes)
				.await?;
		let notes =
			list_notes(&self.db.pool, &req, tenant_id, project_id, requested_status, agent_id, now)
				.await?;
		let items = map_list_items(
			notes,
			agent_id,
			non_private_scopes.as_deref(),
			&shared_grants,
			status_for_note_read,
			now,
		);

		Ok(ListResponse { items })
	}
}

fn requested_list_status(requested_status: Option<&String>) -> Option<&str> {
	requested_status.map(|value| value.trim()).filter(|value| !value.is_empty())
}

fn list_non_private_scopes(scope: Option<&String>) -> Option<Vec<String>> {
	if let Some(scope) = scope {
		if scope == "agent_private" {
			return None;
		}

		return Some(vec![scope.to_string()]);
	}

	Some(vec!["project_shared".to_string(), "org_shared".to_string()])
}

fn validate_list_request(
	req: &ListRequest,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	allowed_scopes: &[String],
) -> Result<()> {
	if tenant_id.is_empty() || project_id.is_empty() {
		return Err(Error::InvalidRequest {
			message: "tenant_id and project_id are required.".to_string(),
		});
	}

	if let Some(scope) = req.scope.as_ref()
		&& !allowed_scopes.iter().any(|value| value == scope)
	{
		return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
	}
	if let Some(agent_id) = req.agent_id.as_ref()
		&& agent_id.trim().is_empty()
	{
		return Err(Error::InvalidRequest {
			message: "agent_id must not be empty when provided.".to_string(),
		});
	}

	if req.scope.as_deref() == Some("agent_private") && agent_id.is_empty() {
		return Err(Error::ScopeDenied {
			message: "agent_id is required for agent_private scope.".to_string(),
		});
	}

	Ok(())
}

fn map_list_items(
	notes: Vec<MemoryNote>,
	agent_id: &str,
	non_private_scopes: Option<&[String]>,
	shared_grants: &HashSet<access::SharedSpaceGrantKey>,
	status_for_note_read: bool,
	now: OffsetDateTime,
) -> Vec<ListItem> {
	notes
		.into_iter()
		.filter(|note| {
			let Some(scopes) = non_private_scopes else {
				return true;
			};

			if status_for_note_read {
				return access::note_read_allowed(note, agent_id, scopes, shared_grants, now);
			}

			note.agent_id == agent_id
				|| shared_grants.contains(&crate::access::SharedSpaceGrantKey {
					scope: note.scope.clone(),
					space_owner_agent_id: note.agent_id.clone(),
				})
		})
		.map(|note| ListItem {
			note_id: note.note_id,
			r#type: note.r#type,
			key: note.key,
			scope: note.scope,
			status: note.status,
			text: note.text,
			importance: note.importance,
			confidence: note.confidence,
			updated_at: note.updated_at,
			expires_at: note.expires_at,
			source_ref: note.source_ref,
		})
		.collect()
}

async fn list_shared_grants(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	non_private_scopes: &Option<Vec<String>>,
) -> Result<HashSet<access::SharedSpaceGrantKey>> {
	if non_private_scopes.is_none() || agent_id.is_empty() {
		return Ok(HashSet::new());
	}

	access::load_shared_read_grants(pool, tenant_id, project_id, agent_id).await
}

async fn list_notes(
	pool: &PgPool,
	req: &ListRequest,
	tenant_id: &str,
	project_id: &str,
	requested_status: Option<&str>,
	agent_id: &str,
	now: OffsetDateTime,
) -> Result<Vec<MemoryNote>> {
	let mut builder = QueryBuilder::new(
		"SELECT note_id, tenant_id, project_id, agent_id, scope, type, key, text, importance, confidence, status, created_at, updated_at, expires_at, embedding_version, source_ref, hit_count, last_hit_at \
					FROM memory_notes WHERE tenant_id = ",
	);

	builder.push_bind(tenant_id);
	builder.push(" AND project_id = ");
	builder.push_bind(project_id);

	if let Some(scope) = &req.scope {
		builder.push(" AND scope = ");
		builder.push_bind(scope);

		if scope == "agent_private" {
			builder.push(" AND agent_id = ");
			builder.push_bind(agent_id);
		}
	} else {
		builder.push(" AND scope != ");
		builder.push_bind("agent_private");
	}
	if let Some(status) = requested_status {
		builder.push(" AND status = ");
		builder.push_bind(status);
	} else {
		builder.push(" AND status = ");
		builder.push_bind("active");
	}

	if requested_status.unwrap_or("active").eq_ignore_ascii_case("active") {
		builder.push(" AND (expires_at IS NULL OR expires_at > ");
		builder.push_bind(now);
		builder.push(")");
	}

	if let Some(note_type) = &req.r#type {
		builder.push(" AND type = ");
		builder.push_bind(note_type);
	}

	builder.build_query_as().fetch_all(pool).await.map_err(Into::into)
}
