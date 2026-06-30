use clap::Args;

use crate::args::constants::{
	DEFAULT_ADMIN_URL, DEFAULT_AGENT_ID, DEFAULT_API_URL, DEFAULT_PROJECT_ID, DEFAULT_READ_PROFILE,
	DEFAULT_TENANT_ID,
};

#[derive(Debug, Args)]
pub(crate) struct PublicEndpointArgs {
	/// Public ELF API base URL.
	#[arg(long, env = "ELF_API_URL", default_value = DEFAULT_API_URL)]
	pub(crate) api_url: String,
	/// Optional bearer token for static-key auth.
	#[arg(long, env = "ELF_USER_TOKEN")]
	pub(crate) token: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct AdminEndpointArgs {
	/// Admin ELF API base URL.
	#[arg(long, env = "ELF_ADMIN_URL", default_value = DEFAULT_ADMIN_URL)]
	pub(crate) admin_url: String,
	/// Optional admin bearer token for static-key auth.
	#[arg(long, env = "ELF_ADMIN_TOKEN")]
	pub(crate) admin_token: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub(crate) struct ContextArgs {
	/// Tenant id sent in X-ELF-Tenant-Id.
	#[arg(long, env = "ELF_TENANT_ID", default_value = DEFAULT_TENANT_ID)]
	pub(crate) tenant_id: String,
	/// Project id sent in X-ELF-Project-Id.
	#[arg(long, env = "ELF_PROJECT_ID", default_value = DEFAULT_PROJECT_ID)]
	pub(crate) project_id: String,
	/// Agent id sent in X-ELF-Agent-Id.
	#[arg(long, env = "ELF_AGENT_ID", default_value = DEFAULT_AGENT_ID)]
	pub(crate) agent_id: String,
}

#[derive(Clone, Debug, Args)]
pub(crate) struct ReadContextArgs {
	#[command(flatten)]
	pub(crate) context: ContextArgs,
	/// Read profile sent in X-ELF-Read-Profile.
	#[arg(long, env = "ELF_READ_PROFILE", default_value = DEFAULT_READ_PROFILE)]
	pub(crate) read_profile: String,
}

#[derive(Debug, Args)]
pub(crate) struct OutputArgs {
	/// Pretty-print the JSON output.
	#[arg(long)]
	pub(crate) pretty: bool,
}
