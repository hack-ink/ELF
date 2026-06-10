//! Scoped core memory block APIs.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgExecutor, Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	ElfService, Error, Result,
	access::{self, ORG_PROJECT_ID},
	search,
};
use elf_config::Config;
use elf_domain::english_gate::{self, EnglishGateKind};

/// Core memory blocks response schema identifier.
pub const ELF_CORE_MEMORY_BLOCKS_SCHEMA_V1: &str = "elf.core_memory_blocks/v1";

const MAX_CORE_BLOCK_CONTENT_CHARS: usize = 2_000;

/// Request payload for attached core block readback.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlocksGetRequest {
	/// Tenant that owns the request.
	pub tenant_id: String,
	/// Project context for attachment lookup.
	pub project_id: String,
	/// Agent requesting attached blocks.
	pub agent_id: String,
	/// Read profile whose exact attachments should be returned.
	pub read_profile: String,
}

/// Response payload for attached core block readback.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlocksResponse {
	/// Response schema identifier.
	pub schema: String,
	/// Tenant that owns the request.
	pub tenant_id: String,
	/// Project context for attachment lookup.
	pub project_id: String,
	/// Agent requesting attached blocks.
	pub agent_id: String,
	/// Read profile used for attachment lookup.
	pub read_profile: String,
	/// Attached core blocks visible to the caller.
	pub items: Vec<CoreBlockItem>,
}

/// One attached core memory block.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockItem {
	/// Core block identifier.
	pub block_id: Uuid,
	/// Active attachment identifier that made the block visible.
	pub attachment_id: Uuid,
	/// Tenant that owns the block.
	pub tenant_id: String,
	/// Project that owns the block.
	pub project_id: String,
	/// Agent that owns the block's scope.
	pub agent_id: String,
	/// Scope key for the block.
	pub scope: String,
	/// Stable block key.
	pub key: String,
	/// Human-readable block title.
	pub title: String,
	/// Small always-attached context payload.
	pub content: String,
	/// Structured source/provenance metadata for the block.
	pub source_ref: Value,
	/// Lifecycle status for the block.
	pub status: String,
	#[serde(with = "crate::time_serde")]
	/// Last block update timestamp.
	pub updated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Attachment creation timestamp.
	pub attached_at: OffsetDateTime,
	/// Agent that created the attachment.
	pub attached_by_agent_id: String,
	/// Append-only block and attachment audit events.
	pub audit_history: Vec<CoreBlockAuditEvent>,
}

/// One core block audit event.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockAuditEvent {
	/// Audit event identifier.
	pub event_id: Uuid,
	/// Block identifier affected by the event.
	pub block_id: Uuid,
	/// Attachment identifier affected by the event, when applicable.
	pub attachment_id: Option<Uuid>,
	/// Agent that performed the event.
	pub actor_agent_id: String,
	/// Event type.
	pub event_type: String,
	/// Attachment target agent, when applicable.
	pub target_agent_id: Option<String>,
	/// Attachment read profile, when applicable.
	pub read_profile: Option<String>,
	/// Optional previous state snapshot.
	pub prev_snapshot: Option<Value>,
	/// Optional new state snapshot.
	pub new_snapshot: Option<Value>,
	/// Human-readable event reason.
	pub reason: String,
	#[serde(with = "crate::time_serde")]
	/// Event timestamp.
	pub ts: OffsetDateTime,
}

/// Request payload for creating or updating a core block through admin APIs.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockUpsertRequest {
	/// Tenant that owns the request.
	pub tenant_id: String,
	/// Project context for the block.
	pub project_id: String,
	/// Agent creating or updating the block.
	pub agent_id: String,
	/// Existing block id to update. Omit to create.
	pub block_id: Option<Uuid>,
	/// Scope key for the block.
	pub scope: String,
	/// Stable block key.
	pub key: String,
	/// Human-readable block title.
	pub title: String,
	/// Small always-attached context payload.
	pub content: String,
	/// Structured source/provenance metadata for the block.
	pub source_ref: Value,
	/// Optional audit reason.
	pub reason: Option<String>,
}

/// Response payload for core block creation or update.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockUpsertResponse {
	/// Stored block record.
	pub block: CoreBlockRecord,
}

/// Core block record returned by admin mutation APIs.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockRecord {
	/// Core block identifier.
	pub block_id: Uuid,
	/// Tenant that owns the block.
	pub tenant_id: String,
	/// Project that owns the block.
	pub project_id: String,
	/// Agent that owns the block's scope.
	pub agent_id: String,
	/// Scope key for the block.
	pub scope: String,
	/// Stable block key.
	pub key: String,
	/// Human-readable block title.
	pub title: String,
	/// Small always-attached context payload.
	pub content: String,
	/// Structured source/provenance metadata for the block.
	pub source_ref: Value,
	/// Lifecycle status for the block.
	pub status: String,
	#[serde(with = "crate::time_serde")]
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Request payload for attaching a block to an agent/read-profile pair.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockAttachRequest {
	/// Tenant that owns the request.
	pub tenant_id: String,
	/// Project context for the attachment.
	pub project_id: String,
	/// Agent creating the attachment.
	pub agent_id: String,
	/// Block to attach.
	pub block_id: Uuid,
	/// Target agent that should receive the block.
	pub target_agent_id: String,
	/// Exact read profile for the attachment.
	pub read_profile: String,
	/// Optional audit reason.
	pub reason: Option<String>,
}

/// Response payload for attaching a core block.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockAttachResponse {
	/// Attachment identifier.
	pub attachment_id: Uuid,
	/// Block identifier.
	pub block_id: Uuid,
	/// Target agent for the attachment.
	pub target_agent_id: String,
	/// Exact read profile for the attachment.
	pub read_profile: String,
	/// Agent that created the attachment.
	pub attached_by_agent_id: String,
	#[serde(with = "crate::time_serde")]
	/// Attachment timestamp.
	pub attached_at: OffsetDateTime,
}

/// Request payload for detaching a block attachment.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockDetachRequest {
	/// Tenant that owns the request.
	pub tenant_id: String,
	/// Project context for the attachment.
	pub project_id: String,
	/// Agent detaching the block.
	pub agent_id: String,
	/// Attachment to detach.
	pub attachment_id: Uuid,
	/// Optional audit reason.
	pub reason: Option<String>,
}

/// Response payload for detaching a core block.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockDetachResponse {
	/// Attachment identifier.
	pub attachment_id: Uuid,
	/// Whether an active attachment was detached.
	pub detached: bool,
}

#[derive(Clone, Debug, FromRow)]
struct CoreBlockRow {
	block_id: Uuid,
	tenant_id: String,
	project_id: String,
	agent_id: String,
	scope: String,
	key: String,
	title: String,
	content: String,
	source_ref: Value,
	status: String,
	created_at: OffsetDateTime,
	updated_at: OffsetDateTime,
}
impl CoreBlockRow {
	fn into_record(self) -> CoreBlockRecord {
		CoreBlockRecord {
			block_id: self.block_id,
			tenant_id: self.tenant_id,
			project_id: self.project_id,
			agent_id: self.agent_id,
			scope: self.scope,
			key: self.key,
			title: self.title,
			content: self.content,
			source_ref: self.source_ref,
			status: self.status,
			created_at: self.created_at,
			updated_at: self.updated_at,
		}
	}
}

#[derive(Clone, Debug, FromRow)]
struct CoreBlockAttachmentRow {
	attachment_id: Uuid,
	block_id: Uuid,
	tenant_id: String,
	project_id: String,
	agent_id: String,
	read_profile: String,
	attached_by_agent_id: String,
	attached_at: OffsetDateTime,
	detached_by_agent_id: Option<String>,
	detached_at: Option<OffsetDateTime>,
}

#[derive(Clone, Debug, FromRow)]
struct CoreBlockJoinedRow {
	attachment_id: Uuid,
	attachment_agent_id: String,
	attached_by_agent_id: String,
	attached_at: OffsetDateTime,
	block_id: Uuid,
	tenant_id: String,
	project_id: String,
	agent_id: String,
	scope: String,
	key: String,
	title: String,
	content: String,
	source_ref: Value,
	status: String,
	created_at: OffsetDateTime,
	updated_at: OffsetDateTime,
}
impl CoreBlockJoinedRow {
	fn into_item(self, audit_by_block: &HashMap<Uuid, Vec<CoreBlockAuditEvent>>) -> CoreBlockItem {
		let audit_history = audit_by_block.get(&self.block_id).cloned().unwrap_or_else(Vec::new);

		CoreBlockItem {
			block_id: self.block_id,
			attachment_id: self.attachment_id,
			tenant_id: self.tenant_id,
			project_id: self.project_id,
			agent_id: self.agent_id,
			scope: self.scope,
			key: self.key,
			title: self.title,
			content: self.content,
			source_ref: self.source_ref,
			status: self.status,
			updated_at: self.updated_at,
			attached_at: self.attached_at,
			attached_by_agent_id: self.attached_by_agent_id,
			audit_history,
		}
	}
}

#[derive(Clone, Debug, FromRow)]
struct CoreBlockEventRow {
	event_id: Uuid,
	block_id: Uuid,
	attachment_id: Option<Uuid>,
	actor_agent_id: String,
	event_type: String,
	target_agent_id: Option<String>,
	read_profile: Option<String>,
	prev_snapshot: Option<Value>,
	new_snapshot: Option<Value>,
	reason: String,
	ts: OffsetDateTime,
}

struct PreparedGetRequest {
	tenant_id: String,
	project_id: String,
	agent_id: String,
	read_profile: String,
	allowed_scopes: Vec<String>,
}

struct PreparedUpsertRequest {
	tenant_id: String,
	project_id: String,
	agent_id: String,
	block_id: Option<Uuid>,
	scope: String,
	key: String,
	title: String,
	content: String,
	source_ref: Value,
	reason: String,
}

struct PreparedAttachRequest {
	tenant_id: String,
	project_id: String,
	agent_id: String,
	block_id: Uuid,
	target_agent_id: String,
	read_profile: String,
	allowed_scopes: Vec<String>,
	reason: String,
}

struct PreparedDetachRequest {
	tenant_id: String,
	project_id: String,
	agent_id: String,
	attachment_id: Uuid,
	reason: String,
}

struct CoreBlockEventInput<'a> {
	block_id: Uuid,
	attachment_id: Option<Uuid>,
	tenant_id: &'a str,
	project_id: &'a str,
	actor_agent_id: &'a str,
	event_type: &'a str,
	target_agent_id: Option<&'a str>,
	read_profile: Option<&'a str>,
	prev_snapshot: Option<Value>,
	new_snapshot: Option<Value>,
	reason: &'a str,
	ts: OffsetDateTime,
}

impl ElfService {
	/// Returns core memory blocks explicitly attached for one agent/read-profile pair.
	pub async fn core_blocks_get(&self, req: CoreBlocksGetRequest) -> Result<CoreBlocksResponse> {
		let prepared = prepare_get_request(&self.cfg, req)?;
		let rows = fetch_attached_block_rows(
			&self.db.pool,
			prepared.tenant_id.as_str(),
			prepared.project_id.as_str(),
			prepared.agent_id.as_str(),
			prepared.read_profile.as_str(),
		)
		.await?;
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			&self.db.pool,
			prepared.tenant_id.as_str(),
			prepared.project_id.as_str(),
			prepared.agent_id.as_str(),
			prepared.allowed_scopes.iter().any(|scope| scope == "org_shared"),
		)
		.await?;
		let visible_rows = filter_visible_rows(rows, &prepared.allowed_scopes, &shared_grants);
		let block_ids = visible_rows.iter().map(|row| row.block_id).collect::<Vec<_>>();
		let audit_by_block = fetch_audit_history(&self.db.pool, &block_ids).await?;
		let items =
			visible_rows.into_iter().map(|row| row.into_item(&audit_by_block)).collect::<Vec<_>>();

		Ok(CoreBlocksResponse {
			schema: ELF_CORE_MEMORY_BLOCKS_SCHEMA_V1.to_string(),
			tenant_id: prepared.tenant_id,
			project_id: prepared.project_id,
			agent_id: prepared.agent_id,
			read_profile: prepared.read_profile,
			items,
		})
	}

	/// Creates or updates a core memory block and records append-only audit history.
	pub async fn core_block_upsert(
		&self,
		req: CoreBlockUpsertRequest,
	) -> Result<CoreBlockUpsertResponse> {
		let prepared = prepare_upsert_request(&self.cfg, req)?;
		let now = OffsetDateTime::now_utc();
		let mut tx = self.db.pool.begin().await?;
		let (row, prev_snapshot) = match prepared.block_id {
			Some(block_id) => update_core_block(&mut tx, &prepared, block_id, now).await?,
			None => (insert_core_block(&mut tx, &prepared, now).await?, None),
		};

		insert_core_block_event(
			&mut tx,
			CoreBlockEventInput {
				block_id: row.block_id,
				attachment_id: None,
				tenant_id: prepared.tenant_id.as_str(),
				project_id: prepared.project_id.as_str(),
				actor_agent_id: prepared.agent_id.as_str(),
				event_type: if prepared.block_id.is_some() {
					"block_updated"
				} else {
					"block_created"
				},
				target_agent_id: None,
				read_profile: None,
				prev_snapshot,
				new_snapshot: Some(block_snapshot(&row)),
				reason: prepared.reason.as_str(),
				ts: now,
			},
		)
		.await?;

		tx.commit().await?;

		Ok(CoreBlockUpsertResponse { block: row.into_record() })
	}

	/// Attaches an active core block to one exact agent/read-profile pair.
	pub async fn core_block_attach(
		&self,
		req: CoreBlockAttachRequest,
	) -> Result<CoreBlockAttachResponse> {
		let prepared = prepare_attach_request(&self.cfg, req)?;
		let now = OffsetDateTime::now_utc();
		let mut tx = self.db.pool.begin().await?;
		let block = fetch_active_block_for_attachment(&mut tx, &prepared).await?;
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			&mut *tx,
			prepared.tenant_id.as_str(),
			prepared.project_id.as_str(),
			prepared.target_agent_id.as_str(),
			prepared.allowed_scopes.iter().any(|scope| scope == "org_shared"),
		)
		.await?;

		if !block_read_allowed(
			&block,
			prepared.target_agent_id.as_str(),
			&prepared.allowed_scopes,
			&shared_grants,
		) {
			return Err(Error::ScopeDenied {
				message: "Block scope is not allowed for this attachment.".to_string(),
			});
		}

		let attachment = upsert_core_block_attachment(&mut tx, &prepared, now).await?;

		insert_core_block_event(
			&mut tx,
			CoreBlockEventInput {
				block_id: attachment.block_id,
				attachment_id: Some(attachment.attachment_id),
				tenant_id: prepared.tenant_id.as_str(),
				project_id: prepared.project_id.as_str(),
				actor_agent_id: prepared.agent_id.as_str(),
				event_type: "attachment_added",
				target_agent_id: Some(prepared.target_agent_id.as_str()),
				read_profile: Some(prepared.read_profile.as_str()),
				prev_snapshot: None,
				new_snapshot: Some(attachment_snapshot(&attachment)),
				reason: prepared.reason.as_str(),
				ts: now,
			},
		)
		.await?;

		tx.commit().await?;

		Ok(CoreBlockAttachResponse {
			attachment_id: attachment.attachment_id,
			block_id: attachment.block_id,
			target_agent_id: attachment.agent_id,
			read_profile: attachment.read_profile,
			attached_by_agent_id: attachment.attached_by_agent_id,
			attached_at: attachment.attached_at,
		})
	}

	/// Detaches an active core block attachment and records an audit event.
	pub async fn core_block_detach(
		&self,
		req: CoreBlockDetachRequest,
	) -> Result<CoreBlockDetachResponse> {
		let prepared = prepare_detach_request(req)?;
		let now = OffsetDateTime::now_utc();
		let mut tx = self.db.pool.begin().await?;
		let Some(prev) = fetch_active_attachment_for_update(&mut tx, &prepared).await? else {
			tx.commit().await?;

			return Ok(CoreBlockDetachResponse {
				attachment_id: prepared.attachment_id,
				detached: false,
			});
		};
		let updated = detach_core_block_attachment(&mut tx, &prepared, now).await?;

		insert_core_block_event(
			&mut tx,
			CoreBlockEventInput {
				block_id: updated.block_id,
				attachment_id: Some(updated.attachment_id),
				tenant_id: prepared.tenant_id.as_str(),
				project_id: prepared.project_id.as_str(),
				actor_agent_id: prepared.agent_id.as_str(),
				event_type: "attachment_removed",
				target_agent_id: Some(updated.agent_id.as_str()),
				read_profile: Some(updated.read_profile.as_str()),
				prev_snapshot: Some(attachment_snapshot(&prev)),
				new_snapshot: Some(attachment_snapshot(&updated)),
				reason: prepared.reason.as_str(),
				ts: now,
			},
		)
		.await?;

		tx.commit().await?;

		Ok(CoreBlockDetachResponse { attachment_id: updated.attachment_id, detached: true })
	}
}

fn prepare_get_request(cfg: &Config, req: CoreBlocksGetRequest) -> Result<PreparedGetRequest> {
	let tenant_id = normalize_required(req.tenant_id.as_str(), "tenant_id")?;
	let project_id = normalize_required(req.project_id.as_str(), "project_id")?;
	let agent_id = normalize_required(req.agent_id.as_str(), "agent_id")?;
	let read_profile = normalize_required(req.read_profile.as_str(), "read_profile")?;
	let allowed_scopes = search::resolve_read_profile_scopes(cfg, read_profile.as_str())?;

	Ok(PreparedGetRequest { tenant_id, project_id, agent_id, read_profile, allowed_scopes })
}

fn prepare_upsert_request(
	cfg: &Config,
	req: CoreBlockUpsertRequest,
) -> Result<PreparedUpsertRequest> {
	let tenant_id = normalize_required(req.tenant_id.as_str(), "tenant_id")?;
	let requested_project_id = normalize_required(req.project_id.as_str(), "project_id")?;
	let agent_id = normalize_required(req.agent_id.as_str(), "agent_id")?;
	let scope = normalize_required(req.scope.as_str(), "scope")?;
	let key = normalize_required(req.key.as_str(), "key")?;
	let title = normalize_required(req.title.as_str(), "title")?;
	let content = normalize_required(req.content.as_str(), "content")?;
	let reason = req
		.reason
		.as_deref()
		.map(|value| normalize_required(value, "reason"))
		.transpose()?
		.unwrap_or_else(|| "core block upsert".to_string());
	let project_id =
		if scope == "org_shared" { ORG_PROJECT_ID.to_string() } else { requested_project_id };

	validate_write_scope(cfg, scope.as_str())?;
	validate_english(key.as_str(), EnglishGateKind::Identifier, "$.key")?;
	validate_english(title.as_str(), EnglishGateKind::NaturalLanguage, "$.title")?;
	validate_english(content.as_str(), EnglishGateKind::NaturalLanguage, "$.content")?;
	validate_source_ref(&req.source_ref)?;

	if content.chars().count() > MAX_CORE_BLOCK_CONTENT_CHARS {
		return Err(Error::InvalidRequest { message: "content is too long.".to_string() });
	}

	Ok(PreparedUpsertRequest {
		tenant_id,
		project_id,
		agent_id,
		block_id: req.block_id,
		scope,
		key,
		title,
		content,
		source_ref: req.source_ref,
		reason,
	})
}

fn prepare_attach_request(
	cfg: &Config,
	req: CoreBlockAttachRequest,
) -> Result<PreparedAttachRequest> {
	let tenant_id = normalize_required(req.tenant_id.as_str(), "tenant_id")?;
	let project_id = normalize_required(req.project_id.as_str(), "project_id")?;
	let agent_id = normalize_required(req.agent_id.as_str(), "agent_id")?;
	let target_agent_id = normalize_required(req.target_agent_id.as_str(), "target_agent_id")?;
	let read_profile = normalize_required(req.read_profile.as_str(), "read_profile")?;
	let allowed_scopes = search::resolve_read_profile_scopes(cfg, read_profile.as_str())?;
	let reason = req
		.reason
		.as_deref()
		.map(|value| normalize_required(value, "reason"))
		.transpose()?
		.unwrap_or_else(|| "core block attachment".to_string());

	validate_english(target_agent_id.as_str(), EnglishGateKind::Identifier, "$.target_agent_id")?;

	Ok(PreparedAttachRequest {
		tenant_id,
		project_id,
		agent_id,
		block_id: req.block_id,
		target_agent_id,
		read_profile,
		allowed_scopes,
		reason,
	})
}

fn prepare_detach_request(req: CoreBlockDetachRequest) -> Result<PreparedDetachRequest> {
	let tenant_id = normalize_required(req.tenant_id.as_str(), "tenant_id")?;
	let project_id = normalize_required(req.project_id.as_str(), "project_id")?;
	let agent_id = normalize_required(req.agent_id.as_str(), "agent_id")?;
	let reason = req
		.reason
		.as_deref()
		.map(|value| normalize_required(value, "reason"))
		.transpose()?
		.unwrap_or_else(|| "core block detach".to_string());

	Ok(PreparedDetachRequest {
		tenant_id,
		project_id,
		agent_id,
		attachment_id: req.attachment_id,
		reason,
	})
}

fn filter_visible_rows(
	rows: Vec<CoreBlockJoinedRow>,
	allowed_scopes: &[String],
	shared_grants: &HashSet<access::SharedSpaceGrantKey>,
) -> Vec<CoreBlockJoinedRow> {
	rows.into_iter()
		.filter(|row| {
			let block = CoreBlockRow {
				block_id: row.block_id,
				tenant_id: row.tenant_id.clone(),
				project_id: row.project_id.clone(),
				agent_id: row.agent_id.clone(),
				scope: row.scope.clone(),
				key: row.key.clone(),
				title: row.title.clone(),
				content: row.content.clone(),
				source_ref: row.source_ref.clone(),
				status: row.status.clone(),
				created_at: row.created_at,
				updated_at: row.updated_at,
			};

			block_read_allowed(
				&block,
				row.attachment_agent_id.as_str(),
				allowed_scopes,
				shared_grants,
			)
		})
		.collect()
}

fn block_read_allowed(
	block: &CoreBlockRow,
	requester_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<access::SharedSpaceGrantKey>,
) -> bool {
	if block.status != "active" {
		return false;
	}
	if !allowed_scopes.iter().any(|scope| scope == &block.scope) {
		return false;
	}
	if block.scope == "agent_private" {
		return block.agent_id == requester_agent_id;
	}
	if !matches!(block.scope.as_str(), "project_shared" | "org_shared") {
		return false;
	}
	if block.agent_id == requester_agent_id {
		return true;
	}

	shared_grants.contains(&access::SharedSpaceGrantKey {
		scope: block.scope.clone(),
		space_owner_agent_id: block.agent_id.clone(),
	})
}

fn block_snapshot(block: &CoreBlockRow) -> Value {
	serde_json::json!({
		"block_id": block.block_id,
		"tenant_id": block.tenant_id,
		"project_id": block.project_id,
		"agent_id": block.agent_id,
		"scope": block.scope,
		"key": block.key,
		"title": block.title,
		"content": block.content,
		"source_ref": block.source_ref,
		"status": block.status,
		"created_at": block.created_at,
		"updated_at": block.updated_at,
	})
}

fn attachment_snapshot(attachment: &CoreBlockAttachmentRow) -> Value {
	serde_json::json!({
		"attachment_id": attachment.attachment_id,
		"block_id": attachment.block_id,
		"tenant_id": attachment.tenant_id,
		"project_id": attachment.project_id,
		"agent_id": attachment.agent_id,
		"read_profile": attachment.read_profile,
		"attached_by_agent_id": attachment.attached_by_agent_id,
		"attached_at": attachment.attached_at,
		"detached_by_agent_id": attachment.detached_by_agent_id,
		"detached_at": attachment.detached_at,
	})
}

fn normalize_required(raw: &str, field: &str) -> Result<String> {
	let trimmed = raw.trim();

	if trimmed.is_empty() {
		return Err(Error::InvalidRequest { message: format!("{field} is required.") });
	}

	Ok(trimmed.to_string())
}

fn validate_write_scope(cfg: &Config, scope: &str) -> Result<()> {
	if !cfg.scopes.allowed.iter().any(|allowed| allowed == scope) {
		return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
	}

	let write_allowed = match scope {
		"agent_private" => cfg.scopes.write_allowed.agent_private,
		"project_shared" => cfg.scopes.write_allowed.project_shared,
		"org_shared" => cfg.scopes.write_allowed.org_shared,
		_ => false,
	};

	if !write_allowed {
		return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
	}

	Ok(())
}

fn validate_english(input: &str, kind: EnglishGateKind, field: &str) -> Result<()> {
	english_gate::english_gate(input, kind)
		.map_err(|_| Error::NonEnglishInput { field: field.to_string() })
}

fn validate_source_ref(source_ref: &Value) -> Result<()> {
	if !source_ref.is_object() {
		return Err(Error::InvalidRequest {
			message: "source_ref must be a JSON object.".to_string(),
		});
	}

	Ok(())
}

async fn insert_core_block(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedUpsertRequest,
	now: OffsetDateTime,
) -> Result<CoreBlockRow> {
	ensure_no_active_key_conflict(tx, req, None).await?;

	sqlx::query_as::<_, CoreBlockRow>(
		"\
INSERT INTO core_memory_blocks (
	block_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	key,
	title,
	content,
	source_ref,
	status,
	created_at,
	updated_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'active', $10, $10)
RETURNING *",
	)
	.bind(Uuid::new_v4())
	.bind(req.tenant_id.as_str())
	.bind(req.project_id.as_str())
	.bind(req.agent_id.as_str())
	.bind(req.scope.as_str())
	.bind(req.key.as_str())
	.bind(req.title.as_str())
	.bind(req.content.as_str())
	.bind(&req.source_ref)
	.bind(now)
	.fetch_one(&mut **tx)
	.await
	.map_err(Into::into)
}

async fn update_core_block(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedUpsertRequest,
	block_id: Uuid,
	now: OffsetDateTime,
) -> Result<(CoreBlockRow, Option<Value>)> {
	let prev = fetch_owned_block_for_update(tx, req, block_id).await?;
	let prev_snapshot = Some(block_snapshot(&prev));

	ensure_no_active_key_conflict(tx, req, Some(block_id)).await?;

	let row = sqlx::query_as::<_, CoreBlockRow>(
		"\
UPDATE core_memory_blocks
SET
	key = $6,
	title = $7,
	content = $8,
	source_ref = $9,
	updated_at = $10
WHERE block_id = $1
	AND tenant_id = $2
	AND project_id = $3
	AND agent_id = $4
	AND scope = $5
	AND status = 'active'
RETURNING *",
	)
	.bind(block_id)
	.bind(req.tenant_id.as_str())
	.bind(req.project_id.as_str())
	.bind(req.agent_id.as_str())
	.bind(req.scope.as_str())
	.bind(req.key.as_str())
	.bind(req.title.as_str())
	.bind(req.content.as_str())
	.bind(&req.source_ref)
	.bind(now)
	.fetch_optional(&mut **tx)
	.await?
	.ok_or_else(|| Error::NotFound { message: "Core block not found.".to_string() })?;

	Ok((row, prev_snapshot))
}

async fn fetch_owned_block_for_update(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedUpsertRequest,
	block_id: Uuid,
) -> Result<CoreBlockRow> {
	sqlx::query_as::<_, CoreBlockRow>(
		"\
SELECT *
FROM core_memory_blocks
WHERE block_id = $1
	AND tenant_id = $2
	AND project_id = $3
	AND agent_id = $4
	AND scope = $5
	AND status = 'active'
FOR UPDATE",
	)
	.bind(block_id)
	.bind(req.tenant_id.as_str())
	.bind(req.project_id.as_str())
	.bind(req.agent_id.as_str())
	.bind(req.scope.as_str())
	.fetch_optional(&mut **tx)
	.await?
	.ok_or_else(|| Error::NotFound { message: "Core block not found.".to_string() })
}

async fn ensure_no_active_key_conflict(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedUpsertRequest,
	block_id: Option<Uuid>,
) -> Result<()> {
	let conflict: Option<Uuid> = sqlx::query_scalar(
		"\
SELECT block_id
FROM core_memory_blocks
WHERE tenant_id = $1
	AND project_id = $2
	AND agent_id = $3
	AND scope = $4
	AND key = $5
	AND status = 'active'
	AND ($6::uuid IS NULL OR block_id <> $6)
LIMIT 1",
	)
	.bind(req.tenant_id.as_str())
	.bind(req.project_id.as_str())
	.bind(req.agent_id.as_str())
	.bind(req.scope.as_str())
	.bind(req.key.as_str())
	.bind(block_id)
	.fetch_optional(&mut **tx)
	.await?;

	if conflict.is_some() {
		return Err(Error::Conflict { message: "Core block key already exists.".to_string() });
	}

	Ok(())
}

async fn fetch_active_block_for_attachment(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedAttachRequest,
) -> Result<CoreBlockRow> {
	sqlx::query_as::<_, CoreBlockRow>(
		"\
SELECT *
FROM core_memory_blocks
WHERE block_id = $1
	AND tenant_id = $2
	AND status = 'active'
	AND (
		project_id = $3
		OR (project_id = $4 AND scope = 'org_shared')
	)",
	)
	.bind(req.block_id)
	.bind(req.tenant_id.as_str())
	.bind(req.project_id.as_str())
	.bind(ORG_PROJECT_ID)
	.fetch_optional(&mut **tx)
	.await?
	.ok_or_else(|| Error::NotFound { message: "Core block not found.".to_string() })
}

async fn upsert_core_block_attachment(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedAttachRequest,
	now: OffsetDateTime,
) -> Result<CoreBlockAttachmentRow> {
	sqlx::query_as::<_, CoreBlockAttachmentRow>(
		"\
INSERT INTO core_memory_block_attachments (
	attachment_id,
	block_id,
	tenant_id,
	project_id,
	agent_id,
	read_profile,
	attached_by_agent_id,
	attached_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
ON CONFLICT (tenant_id, project_id, agent_id, read_profile, block_id)
WHERE detached_at IS NULL
DO UPDATE
SET
	attached_by_agent_id = EXCLUDED.attached_by_agent_id,
	attached_at = EXCLUDED.attached_at,
	detached_by_agent_id = NULL,
	detached_at = NULL
RETURNING *",
	)
	.bind(Uuid::new_v4())
	.bind(req.block_id)
	.bind(req.tenant_id.as_str())
	.bind(req.project_id.as_str())
	.bind(req.target_agent_id.as_str())
	.bind(req.read_profile.as_str())
	.bind(req.agent_id.as_str())
	.bind(now)
	.fetch_one(&mut **tx)
	.await
	.map_err(Into::into)
}

async fn fetch_active_attachment_for_update(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedDetachRequest,
) -> Result<Option<CoreBlockAttachmentRow>> {
	sqlx::query_as::<_, CoreBlockAttachmentRow>(
		"\
SELECT *
FROM core_memory_block_attachments
WHERE attachment_id = $1
	AND tenant_id = $2
	AND project_id = $3
	AND detached_at IS NULL
FOR UPDATE",
	)
	.bind(req.attachment_id)
	.bind(req.tenant_id.as_str())
	.bind(req.project_id.as_str())
	.fetch_optional(&mut **tx)
	.await
	.map_err(Into::into)
}

async fn detach_core_block_attachment(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedDetachRequest,
	now: OffsetDateTime,
) -> Result<CoreBlockAttachmentRow> {
	sqlx::query_as::<_, CoreBlockAttachmentRow>(
		"\
UPDATE core_memory_block_attachments
SET
	detached_by_agent_id = $4,
	detached_at = $5
WHERE attachment_id = $1
	AND tenant_id = $2
	AND project_id = $3
	AND detached_at IS NULL
RETURNING *",
	)
	.bind(req.attachment_id)
	.bind(req.tenant_id.as_str())
	.bind(req.project_id.as_str())
	.bind(req.agent_id.as_str())
	.bind(now)
	.fetch_one(&mut **tx)
	.await
	.map_err(Into::into)
}

async fn fetch_attached_block_rows<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	read_profile: &str,
) -> Result<Vec<CoreBlockJoinedRow>>
where
	E: PgExecutor<'e>,
{
	sqlx::query_as::<_, CoreBlockJoinedRow>(
		"\
SELECT
	a.attachment_id,
	a.agent_id AS attachment_agent_id,
	a.attached_by_agent_id,
	a.attached_at,
	b.block_id,
	b.tenant_id,
	b.project_id,
	b.agent_id,
	b.scope,
	b.key,
	b.title,
	b.content,
	b.source_ref,
	b.status,
	b.created_at,
	b.updated_at
FROM core_memory_block_attachments a
JOIN core_memory_blocks b ON b.block_id = a.block_id
WHERE a.tenant_id = $1
	AND a.project_id = $2
	AND a.agent_id = $3
	AND a.read_profile = $4
	AND a.detached_at IS NULL
	AND b.status = 'active'
ORDER BY a.attached_at ASC, b.key ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(agent_id)
	.bind(read_profile)
	.fetch_all(executor)
	.await
	.map_err(Into::into)
}

async fn fetch_audit_history<'e, E>(
	executor: E,
	block_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<CoreBlockAuditEvent>>>
where
	E: PgExecutor<'e>,
{
	if block_ids.is_empty() {
		return Ok(HashMap::new());
	}

	let rows = sqlx::query_as::<_, CoreBlockEventRow>(
		"\
SELECT
	event_id,
	block_id,
	attachment_id,
	actor_agent_id,
	event_type,
	target_agent_id,
	read_profile,
	prev_snapshot,
	new_snapshot,
	reason,
	ts
FROM core_memory_block_events
WHERE block_id = ANY($1)
ORDER BY ts ASC, event_id ASC",
	)
	.bind(block_ids)
	.fetch_all(executor)
	.await?;
	let mut by_block: HashMap<Uuid, Vec<CoreBlockAuditEvent>> = HashMap::new();

	for row in rows {
		by_block.entry(row.block_id).or_default().push(CoreBlockAuditEvent {
			event_id: row.event_id,
			block_id: row.block_id,
			attachment_id: row.attachment_id,
			actor_agent_id: row.actor_agent_id,
			event_type: row.event_type,
			target_agent_id: row.target_agent_id,
			read_profile: row.read_profile,
			prev_snapshot: row.prev_snapshot,
			new_snapshot: row.new_snapshot,
			reason: row.reason,
			ts: row.ts,
		});
	}

	Ok(by_block)
}

async fn insert_core_block_event(
	tx: &mut Transaction<'_, Postgres>,
	event: CoreBlockEventInput<'_>,
) -> Result<()> {
	sqlx::query(
		"\
INSERT INTO core_memory_block_events (
	event_id,
	block_id,
	attachment_id,
	tenant_id,
	project_id,
	actor_agent_id,
	event_type,
	target_agent_id,
	read_profile,
	prev_snapshot,
	new_snapshot,
	reason,
	ts
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
	)
	.bind(Uuid::new_v4())
	.bind(event.block_id)
	.bind(event.attachment_id)
	.bind(event.tenant_id)
	.bind(event.project_id)
	.bind(event.actor_agent_id)
	.bind(event.event_type)
	.bind(event.target_agent_id)
	.bind(event.read_profile)
	.bind(event.prev_snapshot)
	.bind(event.new_snapshot)
	.bind(event.reason)
	.bind(event.ts)
	.execute(&mut **tx)
	.await?;

	Ok(())
}
