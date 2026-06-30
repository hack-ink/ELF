use std::{
	sync::{Arc, Mutex},
	time::Duration,
};

use axum::{
	Json, Router,
	extract::State,
	http::{Method, Uri},
	routing,
};
use serde_json::{Map, Value};
use tokio::{
	net::TcpListener,
	sync::{
		oneshot,
		oneshot::{Receiver, Sender},
	},
	time,
};

use crate::app::{
	McpAuthState,
	server::{ElfContextHeaders, ElfMcp},
};
use elf_config::McpContext;

type RequestRecorder = Arc<Mutex<Option<Sender<RecordedRequest>>>>;

struct RecordedRequest {
	method: Method,
	path: String,
	body: Value,
}

#[tokio::test]
async fn recall_debug_panel_rejects_context_override_params() {
	let context = McpContext {
		tenant_id: "tenant-a".to_string(),
		project_id: "project-a".to_string(),
		agent_id: "agent-a".to_string(),
		read_profile: "private_plus_project".to_string(),
	};
	let mcp = ElfMcp::new(
		"http://127.0.0.1:1".to_string(),
		"http://127.0.0.1:1".to_string(),
		ElfContextHeaders::new(&context),
		McpAuthState::Off,
	);
	let params =
		Map::from_iter([("tenant_id".to_string(), Value::String("tenant-override".to_string()))]);
	let result = mcp.elf_recall_debug_panel(params).await;
	let err = result.expect_err("context override params must fail before forwarding.");

	assert!(format!("{err:?}").contains("tenant_id"));
}

#[tokio::test]
async fn default_ingestion_profile_set_uses_put_admin_default_path() {
	let (admin_base, received) = spawn_recording_admin_server().await;
	let context = McpContext {
		tenant_id: "tenant-a".to_string(),
		project_id: "project-a".to_string(),
		agent_id: "agent-a".to_string(),
		read_profile: "private_plus_project".to_string(),
	};
	let mcp = ElfMcp::new(
		"http://127.0.0.1:9000".to_string(),
		admin_base,
		ElfContextHeaders::new(&context),
		McpAuthState::Off,
	);
	let params = Map::from_iter([
		("profile_id".to_string(), Value::String("profile-a".to_string())),
		("version".to_string(), Value::Number(2.into())),
	]);
	let result = mcp.elf_admin_events_ingestion_profile_default_set(params).await;

	assert!(result.is_ok(), "default setter should forward successfully: {result:?}");

	let request = receive_recorded_request(received).await;

	assert_eq!(request.method, Method::PUT);
	assert_eq!(request.path, "/v2/admin/events/ingestion-profiles/default");
	assert_eq!(request.body.get("profile_id").and_then(Value::as_str), Some("profile-a"));
	assert_eq!(request.body.get("version").and_then(Value::as_i64), Some(2));
}

async fn spawn_recording_admin_server() -> (String, Receiver<RecordedRequest>) {
	let (tx, rx) = oneshot::channel();
	let app = Router::new()
		.route("/v2/admin/events/ingestion-profiles/default", routing::any(record_request))
		.with_state(Arc::new(Mutex::new(Some(tx))));
	let listener = match TcpListener::bind("127.0.0.1:0").await {
		Ok(listener) => listener,
		Err(err) => panic!("Failed to bind MCP recording admin server: {err}."),
	};
	let addr = match listener.local_addr() {
		Ok(addr) => addr,
		Err(err) => panic!("Failed to read MCP recording admin server address: {err}."),
	};

	tokio::spawn(async move {
		if let Err(err) = axum::serve(listener, app).await {
			panic!("MCP recording admin server failed: {err}.");
		}
	});

	(format!("http://{addr}"), rx)
}

async fn record_request(
	State(recorder): State<RequestRecorder>,
	method: Method,
	uri: Uri,
	Json(body): Json<Value>,
) -> Json<Value> {
	let mut sender = match recorder.lock() {
		Ok(sender) => sender,
		Err(err) => panic!("MCP recording admin server mutex was poisoned: {err}."),
	};

	if let Some(tx) = sender.take() {
		let _ = tx.send(RecordedRequest { method, path: uri.path().to_string(), body });
	}

	Json(serde_json::json!({ "ok": true }))
}

async fn receive_recorded_request(received: Receiver<RecordedRequest>) -> RecordedRequest {
	match time::timeout(Duration::from_secs(3), received).await {
		Ok(Ok(request)) => request,
		Ok(Err(err)) => panic!("MCP recording admin server closed before recording: {err}."),
		Err(err) => panic!("Timed out waiting for MCP recording admin server: {err}."),
	}
}
