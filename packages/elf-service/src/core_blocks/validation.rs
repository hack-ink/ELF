use std::collections::HashSet;

use serde_json::Value;

use crate::{
	Error, Result,
	access::{self, ORG_PROJECT_ID},
	core_blocks::{
		rows::{CoreBlockAttachmentRow, CoreBlockJoinedRow, CoreBlockRow},
		types::{
			CoreBlockAttachRequest, CoreBlockDetachRequest, CoreBlockUpsertRequest,
			CoreBlocksGetRequest, MAX_CORE_BLOCK_CONTENT_CHARS, PreparedAttachRequest,
			PreparedDetachRequest, PreparedGetRequest, PreparedUpsertRequest,
		},
	},
	search,
};
use elf_config::Config;
use elf_domain::english_gate::{self, EnglishGateKind};

pub(super) fn prepare_get_request(
	cfg: &Config,
	req: CoreBlocksGetRequest,
) -> Result<PreparedGetRequest> {
	let tenant_id = normalize_required(req.tenant_id.as_str(), "tenant_id")?;
	let project_id = normalize_required(req.project_id.as_str(), "project_id")?;
	let agent_id = normalize_required(req.agent_id.as_str(), "agent_id")?;
	let read_profile = normalize_required(req.read_profile.as_str(), "read_profile")?;
	let allowed_scopes = search::resolve_read_profile_scopes(cfg, read_profile.as_str())?;

	Ok(PreparedGetRequest { tenant_id, project_id, agent_id, read_profile, allowed_scopes })
}

pub(super) fn prepare_upsert_request(
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

pub(super) fn prepare_attach_request(
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

pub(super) fn prepare_detach_request(req: CoreBlockDetachRequest) -> Result<PreparedDetachRequest> {
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

pub(super) fn filter_visible_rows(
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

pub(super) fn block_read_allowed(
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

pub(super) fn block_snapshot(block: &CoreBlockRow) -> Value {
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

pub(super) fn attachment_snapshot(attachment: &CoreBlockAttachmentRow) -> Value {
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
