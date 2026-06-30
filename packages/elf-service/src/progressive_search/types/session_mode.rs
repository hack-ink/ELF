use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{Error, progressive_search::types::session::SearchSessionizePath};

/// Search-session mode used by progressive-search APIs.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchSessionMode {
	/// Quick-find session without a stored query plan.
	QuickFind,
	/// Planned-search session with a stored query plan.
	PlannedSearch,
}
impl SearchSessionMode {
	pub(in crate::progressive_search) fn as_str(self) -> &'static str {
		match self {
			Self::QuickFind => "quick_find",
			Self::PlannedSearch => "planned_search",
		}
	}
}

impl FromStr for SearchSessionMode {
	type Err = Error;

	fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
		match value {
			"quick_find" => Ok(Self::QuickFind),
			"planned_search" => Ok(Self::PlannedSearch),
			_ => Err(Error::Storage { message: format!("Unknown search session mode: {value}") }),
		}
	}
}

impl From<SearchSessionizePath> for SearchSessionMode {
	fn from(path: SearchSessionizePath) -> Self {
		match path {
			SearchSessionizePath::Quick => Self::QuickFind,
			SearchSessionizePath::Planned => Self::PlannedSearch,
		}
	}
}
