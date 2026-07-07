use crate::{Deserialize, Serialize, TypedStatus};

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct AdapterReport {
	pub(crate) adapter_id: String,
	pub(crate) name: String,
	pub(crate) behavior: String,
	pub(crate) storage: TypedStatus,
	pub(crate) runtime: TypedStatus,
	pub(crate) notes: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct CaptureIntegrationReport {
	#[serde(default)]
	pub(crate) real: Vec<String>,
	#[serde(default)]
	pub(crate) fixture_backed: Vec<String>,
	#[serde(default)]
	pub(crate) mocked: Vec<String>,
	#[serde(default)]
	pub(crate) blocked: Vec<String>,
	#[serde(default)]
	pub(crate) not_encoded: Vec<String>,
	#[serde(default)]
	pub(crate) notes: Vec<String>,
}
