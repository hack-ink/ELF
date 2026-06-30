use crate::ElfService;
use elf_config::SecurityAuthRole;

pub(in crate::admin_graph_predicates) fn is_super_admin_token_id(
	service: &ElfService,
	token_id: Option<&str>,
) -> bool {
	if service.cfg.security.auth_mode.trim() != "static_keys" {
		return false;
	}

	let Some(token_id) = token_id.map(str::trim).filter(|value| !value.is_empty()) else {
		return false;
	};

	service
		.cfg
		.security
		.auth_keys
		.iter()
		.any(|key| key.token_id == token_id && matches!(key.role, SecurityAuthRole::SuperAdmin))
}
