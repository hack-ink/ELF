use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ElfService, Error, InsertVersionArgs, Result, access, note_snapshot};
use elf_storage::models::MemoryNote;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareScope {
	ProjectShared,
	OrgShared,
}

impl ShareScope {
	fn as_str(&self) -> &'static str {
		match self {
			Self::ProjectShared => "project_shared",
			Self::OrgShared => "org_shared",
		}
	}
}

impl std::fmt::Display for ShareScope {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.as_str().fmt(f)
	}
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GranteeKind {
	Project,
	Agent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublishNoteRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub note_id: Uuid,
	pub scope: ShareScope,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublishNoteResponse {
	pub note_id: Uuid,
	pub scope: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnpublishNoteRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub note_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnpublishNoteResponse {
	pub note_id: Uuid,
	pub scope: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpaceGrantUpsertRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: ShareScope,
	pub grantee_kind: GranteeKind,
	pub grantee_agent_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpaceGrantUpsertResponse {
	pub scope: String,
	pub grantee_kind: GranteeKind,
	pub grantee_agent_id: Option<String>,
	pub granted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpaceGrantRevokeRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: ShareScope,
	pub grantee_kind: GranteeKind,
	pub grantee_agent_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpaceGrantRevokeResponse {
	pub revoked: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpaceGrantsListRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: ShareScope,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpaceGrantItem {
	pub scope: ShareScope,
	pub grantee_kind: GranteeKind,
	pub grantee_agent_id: Option<String>,
	pub granted_by_agent_id: String,
	pub granted_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpaceGrantsListResponse {
	pub grants: Vec<SpaceGrantItem>,
}

impl ElfService {
	pub async fn publish_note(&self, req: PublishNoteRequest) -> Result<PublishNoteResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();
		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let mut tx = self.db.pool.begin().await?;
		let mut note: MemoryNote = sqlx::query_as::<_, MemoryNote>(
			"\
SELECT *
FROM memory_notes
WHERE note_id = $1
	AND tenant_id = $2
	AND project_id = $3
FOR UPDATE",
		)
		.bind(req.note_id)
		.bind(tenant_id)
		.bind(project_id)
		.fetch_optional(&mut *tx)
		.await?
		.ok_or_else(|| Error::InvalidRequest { message: "Note not found.".to_string() })?;

		if note.agent_id != agent_id {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		}
		if note.status != "active" {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		}
		if note.expires_at.map(|ts| ts <= time::OffsetDateTime::now_utc()).unwrap_or(false) {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		}

		let scope = req.scope.as_str();
		let scope_allowed = match scope {
			"project_shared" => self.cfg.scopes.write_allowed.project_shared,
			"org_shared" => self.cfg.scopes.write_allowed.org_shared,
			_ => false,
		};
		if !scope_allowed {
			return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
		}

		access::ensure_active_project_scope_grant(&mut *tx, tenant_id, project_id, scope, agent_id)
			.await?;

		if note.scope == scope {
			return Ok(PublishNoteResponse { note_id: note.note_id, scope: note.scope });
		}

		let now = time::OffsetDateTime::now_utc();
		let prev_snapshot = crate::note_snapshot(&note);
		note.scope = scope.to_string();
		note.updated_at = now;

		crate::insert_version(
			&mut *tx,
			InsertVersionArgs {
				note_id: note.note_id,
				op: "PUBLISH",
				prev_snapshot: Some(prev_snapshot),
				new_snapshot: Some(crate::note_snapshot(&note)),
				reason: "publish_note",
				actor: agent_id,
				ts: now,
			},
		)
		.await?;
		sqlx::query("UPDATE memory_notes SET scope = $1, updated_at = $2 WHERE note_id = $3")
			.bind(scope)
			.bind(now)
			.bind(note.note_id)
			.execute(&mut *tx)
			.await?;
		crate::enqueue_outbox_tx(&mut *tx, note.note_id, "UPSERT", &note.embedding_version, now)
			.await?;

		tx.commit().await?;

		Ok(PublishNoteResponse { note_id: note.note_id, scope: note.scope })
	}

	pub async fn unpublish_note(&self, req: UnpublishNoteRequest) -> Result<UnpublishNoteResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();
		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let mut tx = self.db.pool.begin().await?;
		let mut note: MemoryNote = sqlx::query_as::<_, MemoryNote>(
			"\
SELECT *
FROM memory_notes
WHERE note_id = $1
	AND tenant_id = $2
	AND project_id = $3
FOR UPDATE",
		)
		.bind(req.note_id)
		.bind(tenant_id)
		.bind(project_id)
		.fetch_optional(&mut *tx)
		.await?
		.ok_or_else(|| Error::InvalidRequest { message: "Note not found.".to_string() })?;

		if note.agent_id != agent_id {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		}
		if note.status != "active" {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		}
		if note.expires_at.map(|ts| ts <= time::OffsetDateTime::now_utc()).unwrap_or(false) {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		}
		if !self.cfg.scopes.write_allowed.agent_private {
			return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
		}
		if note.scope == "agent_private" {
			return Ok(UnpublishNoteResponse { note_id: note.note_id, scope: note.scope });
		}

		let now = time::OffsetDateTime::now_utc();
		let prev_snapshot = note_snapshot(&note);
		note.scope = "agent_private".to_string();
		note.updated_at = now;

		crate::insert_version(
			&mut *tx,
			InsertVersionArgs {
				note_id: note.note_id,
				op: "UNPUBLISH",
				prev_snapshot: Some(prev_snapshot),
				new_snapshot: Some(note_snapshot(&note)),
				reason: "unpublish_note",
				actor: agent_id,
				ts: now,
			},
		)
		.await?;
		sqlx::query("UPDATE memory_notes SET scope = $1, updated_at = $2 WHERE note_id = $3")
			.bind(note.scope.as_str())
			.bind(now)
			.bind(note.note_id)
			.execute(&mut *tx)
			.await?;
		crate::enqueue_outbox_tx(&mut *tx, note.note_id, "UPSERT", &note.embedding_version, now)
			.await?;

		tx.commit().await?;

		Ok(UnpublishNoteResponse { note_id: note.note_id, scope: note.scope })
	}

	pub async fn space_grant_upsert(
		&self,
		req: SpaceGrantUpsertRequest,
	) -> Result<SpaceGrantUpsertResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();
		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let scope = req.scope.as_str();
		let scope_allowed = match scope {
			"project_shared" => self.cfg.scopes.write_allowed.project_shared,
			"org_shared" => self.cfg.scopes.write_allowed.org_shared,
			_ => false,
		};
		if !scope_allowed {
			return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
		}

		if req.grantee_kind == GranteeKind::Agent
			&& req.grantee_agent_id.as_ref().is_none_or(|id| id.trim().is_empty())
		{
			return Err(Error::InvalidRequest {
				message: "grantee_agent_id is required for agent grantee_kind.".to_string(),
			});
		}

		let grantee_agent_id = req
			.grantee_agent_id
			.as_ref()
			.map(|value| value.trim())
			.filter(|value| !value.is_empty())
			.map(ToString::to_string);
		if req.grantee_kind == GranteeKind::Project && grantee_agent_id.is_some() {
			return Err(Error::InvalidRequest {
				message: "grantee_agent_id must be empty for project grantee_kind.".to_string(),
			});
		}
		let grantee_agent_id_ref = grantee_agent_id.as_deref();

		let now = OffsetDateTime::now_utc();
		let grantee_kind = match req.grantee_kind {
			GranteeKind::Project => "project",
			GranteeKind::Agent => "agent",
		};

		if req.grantee_kind == GranteeKind::Project {
			sqlx::query(
				"\
INSERT INTO memory_space_grants (
	grant_id,
tenant_id,
project_id,
scope,
space_owner_agent_id,
grantee_kind,
grantee_agent_id,
granted_by_agent_id,
granted_at
)
VALUES (
$1,
$2,
$3,
$4,
$5,
$6,
$7,
$8,
$9
)
ON CONFLICT (tenant_id, project_id, scope, space_owner_agent_id)
WHERE revoked_at IS NULL AND grantee_kind = 'project'
DO UPDATE
SET
	granted_by_agent_id = EXCLUDED.granted_by_agent_id,
	granted_at = EXCLUDED.granted_at,
	revoked_at = NULL,
	revoked_by_agent_id = NULL",
			)
			.bind(Uuid::new_v4())
			.bind(tenant_id)
			.bind(project_id)
			.bind(scope)
			.bind(agent_id)
			.bind(grantee_kind)
			.bind::<Option<&str>>(None)
			.bind(agent_id)
			.bind(now)
			.execute(&self.db.pool)
			.await?;
		} else {
			sqlx::query(
				"\
INSERT INTO memory_space_grants (
	grant_id,
tenant_id,
project_id,
scope,
space_owner_agent_id,
grantee_kind,
grantee_agent_id,
granted_by_agent_id,
granted_at
)
VALUES (
$1,
$2,
$3,
$4,
$5,
$6,
$7,
$8,
$9
)
ON CONFLICT (tenant_id, project_id, scope, space_owner_agent_id, grantee_agent_id)
WHERE revoked_at IS NULL AND grantee_kind = 'agent'
DO UPDATE
SET
	granted_by_agent_id = EXCLUDED.granted_by_agent_id,
	granted_at = EXCLUDED.granted_at,
	revoked_at = NULL,
	revoked_by_agent_id = NULL",
			)
			.bind(Uuid::new_v4())
			.bind(tenant_id)
			.bind(project_id)
			.bind(scope)
			.bind(agent_id)
			.bind(grantee_kind)
			.bind(grantee_agent_id_ref)
			.bind(agent_id)
			.bind(now)
			.execute(&self.db.pool)
			.await?;
		}

		Ok(SpaceGrantUpsertResponse {
			scope: scope.to_string(),
			grantee_kind: req.grantee_kind,
			grantee_agent_id,
			granted: true,
		})
	}

	pub async fn space_grant_revoke(
		&self,
		req: SpaceGrantRevokeRequest,
	) -> Result<SpaceGrantRevokeResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();
		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let scope = req.scope.as_str();
		let grantee_agent_id = req
			.grantee_agent_id
			.as_deref()
			.map(|value| value.trim())
			.filter(|value| !value.is_empty());
		if req.grantee_kind == GranteeKind::Agent && grantee_agent_id.is_none() {
			return Err(Error::InvalidRequest {
				message: "grantee_agent_id is required for agent grantee_kind.".to_string(),
			});
		}
		if req.grantee_kind == GranteeKind::Project && grantee_agent_id.is_some() {
			return Err(Error::InvalidRequest {
				message: "grantee_agent_id must be empty for project grantee_kind.".to_string(),
			});
		}

		let scope_allowed = match scope {
			"project_shared" => self.cfg.scopes.write_allowed.project_shared,
			"org_shared" => self.cfg.scopes.write_allowed.org_shared,
			_ => false,
		};
		if !scope_allowed {
			return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
		}

		let revocation = sqlx::query(
			"\
UPDATE memory_space_grants
SET revoked_at = $7,
	revoked_by_agent_id = $8
WHERE tenant_id = $1
  AND project_id = $2
  AND scope = $3
  AND space_owner_agent_id = $4
  AND grantee_kind = $5
  AND ((grantee_kind = 'project' AND grantee_agent_id IS NULL)
  OR (grantee_kind = 'agent' AND grantee_agent_id = $6))
  AND revoked_at IS NULL",
		)
		.bind(tenant_id)
		.bind(project_id)
		.bind(scope)
		.bind(agent_id)
		.bind(match req.grantee_kind {
			GranteeKind::Project => "project",
			GranteeKind::Agent => "agent",
		})
		.bind(grantee_agent_id)
		.bind(OffsetDateTime::now_utc())
		.bind(agent_id)
		.execute(&self.db.pool)
		.await?;

		if revocation.rows_affected() == 0 {
			return Err(Error::InvalidRequest { message: "No active grant found.".to_string() });
		}

		Ok(SpaceGrantRevokeResponse { revoked: true })
	}

	pub async fn space_grants_list(
		&self,
		req: SpaceGrantsListRequest,
	) -> Result<SpaceGrantsListResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();
		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}
		let scope = req.scope.as_str();
		let scope_allowed = match scope {
			"project_shared" => self.cfg.scopes.write_allowed.project_shared,
			"org_shared" => self.cfg.scopes.write_allowed.org_shared,
			_ => false,
		};
		if !scope_allowed {
			return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
		}

		#[derive(sqlx::FromRow)]
		struct Row {
			scope: String,
			grantee_kind: String,
			grantee_agent_id: Option<String>,
			granted_by_agent_id: String,
			granted_at: OffsetDateTime,
		}

		let rows = sqlx::query_as::<_, Row>(
			"\
SELECT scope, grantee_kind, grantee_agent_id, granted_by_agent_id, granted_at
FROM memory_space_grants
WHERE tenant_id = $1
  AND project_id = $2
  AND space_owner_agent_id = $3
  AND scope = $4
  AND revoked_at IS NULL
ORDER BY granted_at DESC",
		)
		.bind(tenant_id)
		.bind(project_id)
		.bind(agent_id)
		.bind(scope)
		.fetch_all(&self.db.pool)
		.await?;
		let mut grants = Vec::with_capacity(rows.len());

		for row in rows {
			let grantee_kind = match row.grantee_kind.as_str() {
				"agent" => GranteeKind::Agent,
				"project" => GranteeKind::Project,
				_ => continue,
			};
			let scope = match row.scope.as_str() {
				"project_shared" => ShareScope::ProjectShared,
				"org_shared" => ShareScope::OrgShared,
				_ => continue,
			};

			grants.push(SpaceGrantItem {
				scope,
				grantee_kind,
				grantee_agent_id: row.grantee_agent_id,
				granted_by_agent_id: row.granted_by_agent_id,
				granted_at: row.granted_at,
			});
		}

		Ok(SpaceGrantsListResponse { grants })
	}
}
