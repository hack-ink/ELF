use crate::{Deserialize, Serialize};

use super::{ExternalAdapterReport, ExternalAdapterSummary};

#[derive(Debug, Deserialize)]
pub(crate) struct ExternalAdapterManifest {
	pub(crate) schema: String,
	pub(crate) manifest_id: String,
	pub(crate) docker_isolation: ExternalDockerIsolation,
	#[serde(default)]
	pub(crate) adapters: Vec<ExternalAdapterReport>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct ExternalAdapterSection {
	pub(crate) schema: String,
	pub(crate) manifest_id: String,
	pub(crate) docker_isolation: ExternalDockerIsolation,
	pub(crate) summary: ExternalAdapterSummary,
	#[serde(default)]
	pub(crate) adapters: Vec<ExternalAdapterReport>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct ExternalDockerIsolation {
	pub(crate) default: bool,
	pub(crate) compose_file: String,
	pub(crate) runner: String,
	pub(crate) artifact_dir: String,
	pub(crate) host_global_installs_required: bool,
	#[serde(default)]
	pub(crate) notes: Vec<String>,
}
