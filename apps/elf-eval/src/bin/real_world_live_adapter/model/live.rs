use super::{Deserialize, Map, MaterializationStatus, PathBuf, serde_json};

#[derive(Debug)]
pub(crate) struct LoadedJob {
	pub(crate) path: PathBuf,
	pub(crate) value: serde_json::Value,
	pub(crate) job: LiveJob,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LiveJob {
	pub(crate) schema: String,
	pub(crate) job_id: String,
	pub(crate) suite: String,
	pub(crate) title: String,
	pub(crate) corpus: LiveCorpus,
	pub(crate) prompt: LivePrompt,
	pub(crate) expected_answer: LiveExpectedAnswer,
	#[serde(default)]
	pub(crate) required_evidence: Vec<LiveRequiredEvidence>,
	#[serde(default)]
	pub(crate) encoding: LiveEncoding,
	pub(crate) memory_evolution: Option<LiveMemoryEvolution>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LiveCorpus {
	#[serde(default)]
	pub(crate) items: Vec<LiveCorpusItem>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LiveCorpusItem {
	pub(crate) evidence_id: String,
	pub(crate) text: Option<String>,
	pub(crate) local_ref: Option<String>,
	#[serde(default)]
	pub(crate) capture: LiveCapturePolicy,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct LiveCapturePolicy {
	#[serde(default)]
	pub(crate) action: LiveCaptureAction,

	pub(crate) source_id: Option<String>,

	pub(crate) evidence_binding: Option<String>,

	pub(crate) write_policy: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LivePrompt {
	pub(crate) content: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LiveExpectedAnswer {
	#[serde(default)]
	pub(crate) must_include: Vec<LiveExpectedClaim>,
	#[serde(default)]
	pub(crate) evidence_links: Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LiveRequiredEvidence {
	pub(crate) evidence_id: String,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct LiveMemoryEvolution {
	#[serde(default)]
	pub(crate) current_evidence_ids: Vec<String>,
	#[serde(default)]
	pub(crate) historical_evidence_ids: Vec<String>,
	#[serde(default)]
	pub(crate) tombstone_evidence_ids: Vec<String>,
	#[serde(default)]
	pub(crate) invalidation_evidence_ids: Vec<String>,
	#[serde(default)]
	pub(crate) conflicts: Vec<LiveEvolutionConflict>,
	pub(crate) update_rationale: Option<LiveUpdateRationale>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LiveEvolutionConflict {
	pub(crate) claim_id: String,
	pub(crate) current_evidence_id: String,
	pub(crate) historical_evidence_id: String,
	pub(crate) resolved_by_evidence_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LiveUpdateRationale {
	pub(crate) claim_id: String,
	#[serde(default)]
	pub(crate) evidence_ids: Vec<String>,
	pub(crate) available: bool,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct LiveEncoding {
	pub(crate) status: Option<LiveEncodingStatus>,
	pub(crate) reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LiveCaptureAction {
	#[default]
	Store,
	Exclude,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum LiveExpectedClaim {
	Text(String),
	Object { claim_id: Option<String>, text: String },
}
impl LiveExpectedClaim {
	pub(crate) fn claim_id(&self) -> Option<&str> {
		match self {
			Self::Text(_) => None,
			Self::Object { claim_id, .. } => claim_id.as_deref(),
		}
	}

	pub(crate) fn text(&self) -> &str {
		match self {
			Self::Text(text) => text,
			Self::Object { text, .. } => text,
		}
	}
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LiveEncodingStatus {
	NotEncoded,
	Blocked,
	Incomplete,
}
impl LiveEncodingStatus {
	pub(crate) fn materialization_status(self) -> MaterializationStatus {
		match self {
			Self::NotEncoded => MaterializationStatus::NotEncoded,
			Self::Blocked => MaterializationStatus::Blocked,
			Self::Incomplete => MaterializationStatus::Incomplete,
		}
	}

	pub(crate) fn as_str(self) -> &'static str {
		match self {
			Self::NotEncoded => "not_encoded",
			Self::Blocked => "blocked",
			Self::Incomplete => "incomplete",
		}
	}
}
