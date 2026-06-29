use super::super::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CostReport {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) currency: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) amount: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) input_tokens: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) output_tokens: Option<u64>,
}
