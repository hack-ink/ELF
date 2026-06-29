use super::super::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct DerivedPageArtifact {
	pub(crate) page_id: String,
	pub(crate) page_type: String,
	pub(crate) title: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) path: Option<String>,
	#[serde(default)]
	pub(crate) sections: Vec<DerivedPageSection>,
	#[serde(default)]
	pub(crate) backlinks: Vec<String>,
	#[serde(default)]
	pub(crate) lint_findings: Vec<DerivedPageLintFinding>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) rebuild: Option<DerivedPageRebuild>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) page_version_diff: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct DerivedPageSection {
	pub(crate) section_id: String,
	pub(crate) heading: String,
	pub(crate) role: String,
	pub(crate) content: String,
	#[serde(default)]
	pub(crate) evidence_ids: Vec<String>,
	#[serde(default)]
	pub(crate) timeline_event_ids: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) unsupported_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct DerivedPageLintFinding {
	pub(crate) finding_id: String,
	pub(crate) finding_type: String,
	pub(crate) severity: String,
	pub(crate) text: String,
	#[serde(default)]
	pub(crate) evidence_ids: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) trap_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct DerivedPageRebuild {
	pub(crate) first_hash: String,
	pub(crate) second_hash: String,
	pub(crate) deterministic: bool,
	#[serde(default)]
	pub(crate) allowed_variance: Vec<String>,
}
