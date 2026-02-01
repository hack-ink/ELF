#[path = "../src/routes.rs"]
mod routes;
#[path = "../src/state.rs"]
mod state;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::ServiceExt;

#[tokio::test]
async fn health_ok() {
    let app = routes::router(state::test_state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("Failed to build request."),
        )
        .await
        .expect("Failed to call /health.");
    assert_eq!(response.status(), StatusCode::OK);
}
