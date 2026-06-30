use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Work Journal entry family.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkJournalEntryFamily {
	/// Session log captured alongside source work.
	SessionLog,
	/// Handoff brief for another agent or future session.
	HandoffBrief,
	/// Janitor or cleanup report.
	JanitorReport,
	/// Explicit next step stated in the source.
	ExplicitNextStep,
	/// Inferred next step retained as a non-authoritative hint.
	InferredNextStep,
	/// Option that was considered and rejected.
	RejectedOption,
}
impl WorkJournalEntryFamily {
	/// Returns the canonical API/storage string.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::SessionLog => "session_log",
			Self::HandoffBrief => "handoff_brief",
			Self::JanitorReport => "janitor_report",
			Self::ExplicitNextStep => "explicit_next_step",
			Self::InferredNextStep => "inferred_next_step",
			Self::RejectedOption => "rejected_option",
		}
	}

	pub(in crate::work_journal) fn parse(raw: &str) -> Result<Self> {
		match raw {
			"session_log" => Ok(Self::SessionLog),
			"handoff_brief" => Ok(Self::HandoffBrief),
			"janitor_report" => Ok(Self::JanitorReport),
			"explicit_next_step" => Ok(Self::ExplicitNextStep),
			"inferred_next_step" => Ok(Self::InferredNextStep),
			"rejected_option" => Ok(Self::RejectedOption),
			_ => Err(Error::InvalidRequest {
				message: "family must be one of: session_log, handoff_brief, janitor_report, explicit_next_step, inferred_next_step, rejected_option.".to_string(),
			}),
		}
	}
}
