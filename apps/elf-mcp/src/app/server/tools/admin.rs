use color_eyre::Result;
use rmcp::{
	ErrorData,
	model::{CallToolResult, JsonObject},
};

use crate::app::server::{
	ElfMcp, HttpMethod,
	schemas::{
		admin_ingestion_profile_default_get_schema, admin_ingestion_profile_default_set_schema,
		admin_ingestion_profile_get_schema, admin_ingestion_profile_versions_list_schema,
		admin_ingestion_profiles_create_schema, admin_ingestion_profiles_list_schema,
		admin_memory_history_get_schema, admin_note_provenance_get_schema,
		admin_trace_bundle_get_schema, admin_trace_get_schema, admin_trace_item_get_schema,
		admin_traces_recent_list_schema, admin_trajectory_get_schema,
	},
	support,
};

#[rmcp::tool_router(router = admin_tool_router, vis = "pub(in crate::app::server)")]
impl ElfMcp {
	#[rmcp::tool(
		name = "elf_admin_traces_recent_list",
		description = "List recent traces by tenant/project with optional cursor and filters.",
		input_schema = admin_traces_recent_list_schema()
	)]
	async fn elf_admin_traces_recent_list(
		&self,
		params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		self.forward(HttpMethod::Get, "/v2/admin/traces/recent", params, None).await
	}

	#[rmcp::tool(
		name = "elf_admin_trace_get",
		description = "Fetch trace metadata, items, and optional trajectory summary by trace_id.",
		input_schema = admin_trace_get_schema()
	)]
	async fn elf_admin_trace_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let trace_id = support::take_required_string(&mut params, "trace_id")?;
		let path = format!("/v2/admin/traces/{trace_id}");

		self.forward(HttpMethod::Get, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_admin_trajectory_get",
		description = "Fetch trace trajectory and stage payload by trace_id.",
		input_schema = admin_trajectory_get_schema()
	)]
	async fn elf_admin_trajectory_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let trace_id = support::take_required_string(&mut params, "trace_id")?;
		let path = format!("/v2/admin/trajectories/{trace_id}");

		self.forward(HttpMethod::Get, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_admin_trace_item_get",
		description = "Fetch a trace item explain payload by item_id.",
		input_schema = admin_trace_item_get_schema()
	)]
	async fn elf_admin_trace_item_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let item_id = support::take_required_string(&mut params, "item_id")?;
		let path = format!("/v2/admin/trace-items/{item_id}");

		self.forward(HttpMethod::Get, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_admin_note_provenance_get",
		description = "Fetch provenance bundle and related history for one note.",
		input_schema = admin_note_provenance_get_schema()
	)]
	async fn elf_admin_note_provenance_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let note_id = support::take_required_string(&mut params, "note_id")?;
		let path = format!("/v2/admin/notes/{note_id}/provenance");

		self.forward(HttpMethod::Get, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_admin_memory_history_get",
		description = "Fetch chronological memory history for one note.",
		input_schema = admin_memory_history_get_schema()
	)]
	async fn elf_admin_memory_history_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let note_id = support::take_required_string(&mut params, "note_id")?;
		let path = format!("/v2/admin/notes/{note_id}/history");

		self.forward(HttpMethod::Get, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_admin_trace_bundle_get",
		description = "Fetch trace bundle for replay and diagnostics by trace_id.",
		input_schema = admin_trace_bundle_get_schema()
	)]
	async fn elf_admin_trace_bundle_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let trace_id = support::take_required_string(&mut params, "trace_id")?;
		let path = format!("/v2/admin/traces/{trace_id}/bundle");

		self.forward(HttpMethod::Get, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_admin_events_ingestion_profiles_list",
		description = "List latest ingestion profiles for add_event.",
		input_schema = admin_ingestion_profiles_list_schema()
	)]
	async fn elf_admin_events_ingestion_profiles_list(
		&self,
		_params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		self.forward(
			HttpMethod::Get,
			"/v2/admin/events/ingestion-profiles",
			JsonObject::new(),
			None,
		)
		.await
	}

	#[rmcp::tool(
		name = "elf_admin_events_ingestion_profiles_create",
		description = "Create a new ingestion profile version for add_event.",
		input_schema = admin_ingestion_profiles_create_schema()
	)]
	async fn elf_admin_events_ingestion_profiles_create(
		&self,
		params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		self.forward(HttpMethod::Post, "/v2/admin/events/ingestion-profiles", params, None).await
	}

	#[rmcp::tool(
		name = "elf_admin_events_ingestion_profile_get",
		description = "Get a single ingestion profile by id/version for add_event.",
		input_schema = admin_ingestion_profile_get_schema()
	)]
	async fn elf_admin_events_ingestion_profile_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let profile_id = support::take_required_string(&mut params, "profile_id")?;
		let path = format!("/v2/admin/events/ingestion-profiles/{profile_id}");

		self.forward(HttpMethod::Get, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_admin_events_ingestion_profile_versions_list",
		description = "List all versions of one ingestion profile for add_event.",
		input_schema = admin_ingestion_profile_versions_list_schema()
	)]
	async fn elf_admin_events_ingestion_profile_versions_list(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let profile_id = support::take_required_string(&mut params, "profile_id")?;
		let path = format!("/v2/admin/events/ingestion-profiles/{profile_id}/versions");

		self.forward(HttpMethod::Get, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_admin_events_ingestion_profile_default_get",
		description = "Get the active default ingestion profile for add_event.",
		input_schema = admin_ingestion_profile_default_get_schema()
	)]
	async fn elf_admin_events_ingestion_profile_default_get(
		&self,
		_params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		self.forward(
			HttpMethod::Get,
			"/v2/admin/events/ingestion-profiles/default",
			JsonObject::new(),
			None,
		)
		.await
	}

	#[rmcp::tool(
		name = "elf_admin_events_ingestion_profile_default_set",
		description = "Set the default ingestion profile for add_event.",
		input_schema = admin_ingestion_profile_default_set_schema()
	)]
	pub(in crate::app::server) async fn elf_admin_events_ingestion_profile_default_set(
		&self,
		params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		self.forward(HttpMethod::Put, "/v2/admin/events/ingestion-profiles/default", params, None)
			.await
	}
}
