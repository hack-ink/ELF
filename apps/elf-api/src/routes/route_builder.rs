mod admin;
mod public;

use axum::{Router, extract::DefaultBodyLimit, middleware};

use crate::{
	routes::{self, MAX_DOC_REQUEST_BYTES, MAX_REQUEST_BYTES},
	state::AppState,
};

/// Builds the authenticated public API router.
pub fn router(state: AppState) -> Router {
	let auth_state = state.clone();

	Router::new()
		.merge(routes::contract_router())
		.merge(
			public::public_api_router()
				.with_state(state.clone())
				.layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES)),
		)
		.merge(
			public::docs_api_router()
				.with_state(state)
				.layer(DefaultBodyLimit::max(MAX_DOC_REQUEST_BYTES)),
		)
		.layer(middleware::from_fn_with_state(auth_state, routes::support::api_auth_middleware))
}

/// Builds the authenticated admin API router.
pub fn admin_router(state: AppState) -> Router {
	admin::admin_router(state)
}
