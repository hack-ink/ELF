use crate::{BTreeSet, Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CorpusProfile {
	Synthetic,
	PrivateSanitized,
	GeneratedPublic,
	ExternalAdapter,
}
impl CorpusProfile {
	pub(super) fn as_str(&self) -> &'static str {
		match self {
			Self::Synthetic => "synthetic",
			Self::PrivateSanitized => "private_sanitized",
			Self::GeneratedPublic => "generated_public",
			Self::ExternalAdapter => "external_adapter",
		}
	}
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum ExpectedClaim {
	Text(String),
	Object { claim_id: Option<String>, text: String },
}
impl ExpectedClaim {
	pub(super) fn claim_id(&self) -> Option<&str> {
		match self {
			Self::Text(_) => None,
			Self::Object { claim_id, .. } => claim_id.as_deref(),
		}
	}

	pub(super) fn text(&self) -> &str {
		match self {
			Self::Text(text) => text,
			Self::Object { text, .. } => text,
		}
	}
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum EvidenceLink {
	One(String),
	Many(Vec<String>),
}
impl EvidenceLink {
	pub(super) fn ids(&self) -> BTreeSet<String> {
		match self {
			Self::One(id) => BTreeSet::from([id.clone()]),
			Self::Many(ids) => ids.iter().cloned().collect(),
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ConsolidationReviewAction {
	Apply,
	Discard,
	Defer,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum TypedStatus {
	Pass,
	WrongResult,
	LifecycleFail,
	Incomplete,
	Blocked,
	NotEncoded,
	UnsupportedClaim,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum AdapterCoverageStatus {
	Real,
	Mocked,
	Unsupported,
	Blocked,
	Incomplete,
	WrongResult,
	LifecycleFail,
	Pass,
	NotEncoded,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ElfScenarioPosition {
	Wins,
	Ties,
	Loses,
	Untested,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ScenarioComparisonOutcome {
	Win,
	Tie,
	Loss,
	NotTested,
	Blocked,
	NonGoal,
}
