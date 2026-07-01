use crate::helpers;

#[test]
fn security_auth_keys_require_unique_token_ids() {
	let mut cfg = helpers::base_config();

	cfg.security.auth_mode = "static_keys".to_string();
	cfg.security.auth_keys = vec![
		elf_config::SecurityAuthKey {
			token_id: "k1".to_string(),
			token: "secret-1".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: elf_config::SecurityAuthRole::User,
		},
		elf_config::SecurityAuthKey {
			token_id: "k1".to_string(),
			token: "secret-2".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: elf_config::SecurityAuthRole::Admin,
		},
	];

	let err =
		elf_config::validate(&cfg).expect_err("Expected duplicate token_id validation error.");

	assert!(
		err.to_string().contains("token_id must be unique across security.auth_keys."),
		"Unexpected error: {err}"
	);
}

#[test]
fn security_auth_keys_require_known_read_profile() {
	let mut cfg = helpers::base_config();

	cfg.security.auth_mode = "static_keys".to_string();
	cfg.security.auth_keys = vec![elf_config::SecurityAuthKey {
		token_id: "k1".to_string(),
		token: "secret-1".to_string(),
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: Some("a".to_string()),
		read_profile: "unknown".to_string(),
		role: elf_config::SecurityAuthRole::User,
	}];

	let err =
		elf_config::validate(&cfg).expect_err("Expected auth key read_profile validation error.");

	assert!(
		err.to_string().contains(
			"read_profile must be one of private_only, private_plus_project, or all_scopes."
		),
		"Unexpected error: {err}"
	);
}
