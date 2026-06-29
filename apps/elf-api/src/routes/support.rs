#[path = "support/auth.rs"] mod auth;
#[path = "support/errors.rs"] mod errors;
#[path = "support/headers.rs"] mod headers;
#[path = "support/request_id.rs"] mod request_id;
#[path = "support/scope.rs"] mod scope;
#[path = "support/support_types.rs"] mod support_types;
#[path = "support/time.rs"] mod time;

pub(super) use auth::{
	admin_auth_middleware, api_auth_middleware, effective_token_id,
	require_admin_for_org_shared_writes,
};
pub(super) use errors::{ApiError, json_error};
pub(super) use headers::{RequestContext, required_read_profile};
pub(super) use scope::{format_scope, format_space, parse_space};
pub(super) use support_types::{EntityMemoryQuery, SearchMode, empty_json_object};
pub(super) use time::parse_optional_rfc3339;

#[cfg(test)]
pub(super) use auth::{apply_auth_key_context, resolve_auth_key, sanitize_trusted_token_header};
#[cfg(test)]
pub(super) use request_id::{inject_request_id_into_json_body, parse_request_id_from_headers};
