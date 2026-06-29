use color_eyre::{Result, eyre};
use reqwest::{Client, Method, StatusCode};
use serde_json::Value;

use crate::{
	args::{AddNoteArgs, SearchArgs, StatusArgs},
	http::{self, JsonRequest, redact_url},
	json::{self},
};

pub(crate) async fn run_add_note(client: &Client, args: AddNoteArgs) -> Result<()> {
	let source_ref = json::source_ref(&args.source_id, args.source_ref_json.as_deref())?;
	let body = serde_json::json!({
		"scope": args.scope,
		"notes": [{
			"type": args.note_type,
			"key": args.key,
			"text": args.text,
			"importance": args.importance,
			"confidence": args.confidence,
			"ttl_days": args.ttl_days,
			"source_ref": source_ref,
		}],
	});
	let response = http::request_json(
		client,
		JsonRequest {
			method: Method::POST,
			base_url: &args.endpoint.api_url,
			path: "/v2/notes/ingest",
			token: args.endpoint.token.as_deref(),
			context: Some(&args.context),
			read_profile: None,
			body: Some(&body),
		},
	)
	.await?;
	let output = serde_json::json!({
		"schema": "elf.cli.add_note/v1",
		"request": {
			"api_url": redact_url(&args.endpoint.api_url),
			"tenant_id": args.context.tenant_id,
			"project_id": args.context.project_id,
			"agent_id": args.context.agent_id,
			"scope": body["scope"],
			"source_id": args.source_id,
			"source_ref": body["notes"][0]["source_ref"],
		},
		"response": response,
	});

	json::write_json(&output, args.output.pretty)
}

pub(crate) async fn run_search(client: &Client, args: SearchArgs) -> Result<()> {
	let body = json::search_body(
		args.query,
		args.mode,
		args.top_k,
		args.candidate_k,
		args.payload_level,
		args.filter_json.as_deref(),
	)?;
	let response = http::request_json(
		client,
		JsonRequest {
			method: Method::POST,
			base_url: &args.endpoint.api_url,
			path: "/v2/searches",
			token: args.endpoint.token.as_deref(),
			context: Some(&args.read_context.context),
			read_profile: Some(&args.read_context.read_profile),
			body: Some(&body),
		},
	)
	.await?;
	let output = serde_json::json!({
		"schema": "elf.cli.search/v1",
		"request": {
			"api_url": redact_url(&args.endpoint.api_url),
			"tenant_id": args.read_context.context.tenant_id,
			"project_id": args.read_context.context.project_id,
			"agent_id": args.read_context.context.agent_id,
			"read_profile": args.read_context.read_profile,
			"mode": body["mode"],
			"payload_level": body["payload_level"],
		},
		"trace_id": response.get("trace_id").cloned().unwrap_or(Value::Null),
		"search_id": response.get("search_id").cloned().unwrap_or(Value::Null),
		"response": response,
	});

	json::write_json(&output, args.output.pretty)
}

pub(crate) async fn run_status(client: &Client, args: StatusArgs) -> Result<()> {
	let url = http::join_url(&args.endpoint.api_url, "/health");
	let mut request = client.get(&url);

	if let Some(token) = args.endpoint.token.as_deref() {
		request = request.bearer_auth(token);
	}

	let response = request.send().await?;
	let status = response.status();
	let request_id = http::header_string(response.headers(), "x-elf-request-id");
	let body = response.text().await?;
	let output = serde_json::json!({
		"schema": "elf.cli.status/v1",
		"api": {
			"url": redact_url(&args.endpoint.api_url),
			"healthy": status == StatusCode::OK,
			"status": status.as_u16(),
			"request_id": request_id,
			"body": body,
		},
	});

	json::write_json(&output, args.output.pretty)?;

	if status.is_success() {
		Ok(())
	} else {
		Err(eyre::eyre!("ELF API health check failed with HTTP status {status}."))
	}
}
