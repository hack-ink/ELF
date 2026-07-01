use crate::routes;
use elf_config::SecurityAuthRole;

#[test]
fn require_admin_for_org_shared_writes_denies_user_in_static_keys_mode() {
	let err =
		routes::require_admin_for_org_shared_writes("static_keys", Some(SecurityAuthRole::User))
			.expect_err("Expected forbidden error for non-admin role.");

	assert_eq!(err.status, axum::http::StatusCode::FORBIDDEN);
}

#[test]
fn require_admin_for_org_shared_writes_allows_admin_in_static_keys_mode() {
	routes::require_admin_for_org_shared_writes("static_keys", Some(SecurityAuthRole::Admin))
		.expect("Expected admin role to be allowed.");
}

#[test]
fn require_admin_for_org_shared_writes_allows_superadmin_in_static_keys_mode() {
	routes::require_admin_for_org_shared_writes("static_keys", Some(SecurityAuthRole::SuperAdmin))
		.expect("Expected superadmin role to be allowed.");
}

#[test]
fn require_admin_for_org_shared_writes_allows_non_static_keys_auth_mode() {
	routes::require_admin_for_org_shared_writes("off", None)
		.expect("Expected auth_mode != static_keys.");
}
