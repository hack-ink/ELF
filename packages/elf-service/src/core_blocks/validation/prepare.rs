use crate::{
	Error, Result,
	access::ORG_PROJECT_ID,
	core_blocks::{
		types::{
			CoreBlockAttachRequest, CoreBlockDetachRequest, CoreBlockUpsertRequest,
			CoreBlocksGetRequest,
			constants::MAX_CORE_BLOCK_CONTENT_CHARS,
			prepared::{
				PreparedAttachRequest, PreparedDetachRequest, PreparedGetRequest,
				PreparedUpsertRequest,
			},
		},
		validation::normalize,
	},
	search,
};
use elf_config::Config;
use elf_domain::english_gate::EnglishGateKind;

pub(in crate::core_blocks) fn prepare_get_request(
	cfg: &Config,
	req: CoreBlocksGetRequest,
) -> Result<PreparedGetRequest> {
	let tenant_id = normalize::normalize_required(req.tenant_id.as_str(), "tenant_id")?;
	let project_id = normalize::normalize_required(req.project_id.as_str(), "project_id")?;
	let agent_id = normalize::normalize_required(req.agent_id.as_str(), "agent_id")?;
	let read_profile = normalize::normalize_required(req.read_profile.as_str(), "read_profile")?;
	let allowed_scopes = search::resolve_read_profile_scopes(cfg, read_profile.as_str())?;

	Ok(PreparedGetRequest { tenant_id, project_id, agent_id, read_profile, allowed_scopes })
}

pub(in crate::core_blocks) fn prepare_upsert_request(
	cfg: &Config,
	req: CoreBlockUpsertRequest,
) -> Result<PreparedUpsertRequest> {
	let tenant_id = normalize::normalize_required(req.tenant_id.as_str(), "tenant_id")?;
	let requested_project_id =
		normalize::normalize_required(req.project_id.as_str(), "project_id")?;
	let agent_id = normalize::normalize_required(req.agent_id.as_str(), "agent_id")?;
	let scope = normalize::normalize_required(req.scope.as_str(), "scope")?;
	let key = normalize::normalize_required(req.key.as_str(), "key")?;
	let title = normalize::normalize_required(req.title.as_str(), "title")?;
	let content = normalize::normalize_required(req.content.as_str(), "content")?;
	let reason = req
		.reason
		.as_deref()
		.map(|value| normalize::normalize_required(value, "reason"))
		.transpose()?
		.unwrap_or_else(|| "core block upsert".to_string());
	let project_id =
		if scope == "org_shared" { ORG_PROJECT_ID.to_string() } else { requested_project_id };

	normalize::validate_write_scope(cfg, scope.as_str())?;
	normalize::validate_english(key.as_str(), EnglishGateKind::Identifier, "$.key")?;
	normalize::validate_english(title.as_str(), EnglishGateKind::NaturalLanguage, "$.title")?;
	normalize::validate_english(content.as_str(), EnglishGateKind::NaturalLanguage, "$.content")?;
	normalize::validate_source_ref(&req.source_ref)?;

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

pub(in crate::core_blocks) fn prepare_attach_request(
	cfg: &Config,
	req: CoreBlockAttachRequest,
) -> Result<PreparedAttachRequest> {
	let tenant_id = normalize::normalize_required(req.tenant_id.as_str(), "tenant_id")?;
	let project_id = normalize::normalize_required(req.project_id.as_str(), "project_id")?;
	let agent_id = normalize::normalize_required(req.agent_id.as_str(), "agent_id")?;
	let target_agent_id =
		normalize::normalize_required(req.target_agent_id.as_str(), "target_agent_id")?;
	let read_profile = normalize::normalize_required(req.read_profile.as_str(), "read_profile")?;
	let allowed_scopes = search::resolve_read_profile_scopes(cfg, read_profile.as_str())?;
	let reason = req
		.reason
		.as_deref()
		.map(|value| normalize::normalize_required(value, "reason"))
		.transpose()?
		.unwrap_or_else(|| "core block attachment".to_string());

	normalize::validate_english(
		target_agent_id.as_str(),
		EnglishGateKind::Identifier,
		"$.target_agent_id",
	)?;

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

pub(in crate::core_blocks) fn prepare_detach_request(
	req: CoreBlockDetachRequest,
) -> Result<PreparedDetachRequest> {
	let tenant_id = normalize::normalize_required(req.tenant_id.as_str(), "tenant_id")?;
	let project_id = normalize::normalize_required(req.project_id.as_str(), "project_id")?;
	let agent_id = normalize::normalize_required(req.agent_id.as_str(), "agent_id")?;
	let reason = req
		.reason
		.as_deref()
		.map(|value| normalize::normalize_required(value, "reason"))
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
