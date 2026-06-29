use color_eyre::Result;
use reqwest::{Client, Method};
use serde_json::Value;

use crate::{
	args::{
		AdminPostArgs, AdminSearchArgs, DiagnosticsArgs, DiagnosticsCommand, NoteProvenanceArgs,
		RecentTracesArgs, TraceBundleArgs,
	},
	http::{self, JsonRequest, redact_url},
	json::{self},
};

pub(crate) async fn run_diagnostics(client: &Client, args: DiagnosticsArgs) -> Result<()> {
	match args.command {
		DiagnosticsCommand::QdrantRebuild(args) => run_qdrant_rebuild(client, args).await,
		DiagnosticsCommand::RawSearch(args) => run_raw_search(client, args).await,
		DiagnosticsCommand::RecentTraces(args) => run_recent_traces(client, args).await,
		DiagnosticsCommand::TraceBundle(args) => run_trace_bundle(client, args).await,
		DiagnosticsCommand::NoteProvenance(args) => run_note_provenance(client, args).await,
	}
}

async fn run_qdrant_rebuild(client: &Client, args: AdminPostArgs) -> Result<()> {
	let response = http::request_json(
		client,
		JsonRequest {
			method: Method::POST,
			base_url: &args.endpoint.admin_url,
			path: "/v2/admin/qdrant/rebuild",
			token: args.endpoint.admin_token.as_deref(),
			context: Some(&args.context),
			read_profile: None,
			body: None,
		},
	)
	.await?;
	let output = serde_json::json!({
		"schema": "elf.cli.diagnostics.qdrant_rebuild/v1",
		"admin_url": redact_url(&args.endpoint.admin_url),
		"response": response,
	});

	json::write_json(&output, args.output.pretty)
}

async fn run_raw_search(client: &Client, args: AdminSearchArgs) -> Result<()> {
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
			base_url: &args.endpoint.admin_url,
			path: "/v2/admin/searches/raw",
			token: args.endpoint.admin_token.as_deref(),
			context: Some(&args.read_context.context),
			read_profile: Some(&args.read_context.read_profile),
			body: Some(&body),
		},
	)
	.await?;
	let output = serde_json::json!({
		"schema": "elf.cli.diagnostics.raw_search/v1",
		"request": {
			"admin_url": redact_url(&args.endpoint.admin_url),
			"tenant_id": args.read_context.context.tenant_id,
			"project_id": args.read_context.context.project_id,
			"agent_id": args.read_context.context.agent_id,
			"read_profile": args.read_context.read_profile,
			"mode": body["mode"],
			"payload_level": body["payload_level"],
		},
		"trace_id": response.get("trace_id").cloned().unwrap_or(Value::Null),
		"response": response,
	});

	json::write_json(&output, args.output.pretty)
}

async fn run_recent_traces(client: &Client, args: RecentTracesArgs) -> Result<()> {
	let mut query = Vec::new();

	if let Some(limit) = args.limit {
		query.push(("limit", limit.to_string()));
	}

	let response = http::request_json_query(
		client,
		&args.endpoint.admin_url,
		"/v2/admin/traces/recent",
		args.endpoint.admin_token.as_deref(),
		&args.context,
		&query,
	)
	.await?;
	let output = serde_json::json!({
		"schema": "elf.cli.diagnostics.recent_traces/v1",
		"admin_url": redact_url(&args.endpoint.admin_url),
		"response": response,
	});

	json::write_json(&output, args.output.pretty)
}

async fn run_trace_bundle(client: &Client, args: TraceBundleArgs) -> Result<()> {
	let path = format!("/v2/admin/traces/{}/bundle", args.trace_id);
	let mut query = vec![("mode", args.mode)];

	if let Some(limit) = args.stage_items_limit {
		query.push(("stage_items_limit", limit.to_string()));
	}
	if let Some(limit) = args.candidates_limit {
		query.push(("candidates_limit", limit.to_string()));
	}

	let response = http::request_json_query(
		client,
		&args.endpoint.admin_url,
		&path,
		args.endpoint.admin_token.as_deref(),
		&args.context,
		&query,
	)
	.await?;
	let output = serde_json::json!({
		"schema": "elf.cli.diagnostics.trace_bundle/v1",
		"admin_url": redact_url(&args.endpoint.admin_url),
		"trace_id": response.pointer("/trace/trace_id").cloned().unwrap_or(Value::Null),
		"response": response,
	});

	json::write_json(&output, args.output.pretty)
}

async fn run_note_provenance(client: &Client, args: NoteProvenanceArgs) -> Result<()> {
	let path = format!("/v2/admin/notes/{}/provenance", args.note_id);
	let response = http::request_json_query(
		client,
		&args.endpoint.admin_url,
		&path,
		args.endpoint.admin_token.as_deref(),
		&args.context,
		&[],
	)
	.await?;
	let output = serde_json::json!({
		"schema": "elf.cli.diagnostics.note_provenance/v1",
		"admin_url": redact_url(&args.endpoint.admin_url),
		"note_id": response.pointer("/note/note_id").cloned().unwrap_or(Value::String(args.note_id)),
		"response": response,
	});

	json::write_json(&output, args.output.pretty)
}
