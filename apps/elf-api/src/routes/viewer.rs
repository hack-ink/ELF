use axum::{
	http::{
		HeaderValue,
		header::{CACHE_CONTROL, CONTENT_TYPE},
	},
	response::{IntoResponse, Response},
};

/// Local read-only admin viewer route.
pub const ADMIN_VIEWER_PATH: &str = "/viewer";

pub(in crate::routes) const VIEWER_HTML: &str = include_str!("../../static/viewer.html");

pub(super) async fn admin_viewer() -> Response {
	let mut response = VIEWER_HTML.into_response();

	response
		.headers_mut()
		.insert(CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"));
	response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));

	response
}
