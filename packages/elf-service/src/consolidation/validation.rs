use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;

use crate::{
	Error, Result,
	consolidation::types::{DEFAULT_LIST_LIMIT, MAX_LIST_LIMIT},
};
use elf_domain::consolidation::{
	ConsolidationReviewAction, ConsolidationReviewState, ConsolidationRunState,
	ConsolidationValidationError,
};

pub(super) fn validate_context(tenant_id: &str, project_id: &str, agent_id: &str) -> Result<()> {
	validate_non_empty("tenant_id", tenant_id)?;
	validate_non_empty("project_id", project_id)?;

	validate_non_empty("agent_id", agent_id)
}

pub(super) fn validate_job_kind(job_kind: &str) -> Result<()> {
	validate_non_empty("job_kind", job_kind)?;

	match job_kind {
		"fixture" | "manual" => Ok(()),
		_ => Err(Error::InvalidRequest {
			message: "job_kind must be fixture or manual for consolidation v1.".to_string(),
		}),
	}
}

pub(super) fn validate_object(field: &str, value: &Value) -> Result<()> {
	if matches!(value, Value::Object(_)) {
		Ok(())
	} else {
		Err(Error::InvalidRequest { message: format!("{field} must be a JSON object.") })
	}
}

pub(super) fn validation_error(err: ConsolidationValidationError) -> Error {
	Error::InvalidRequest { message: err.to_string() }
}

pub(super) fn review_steps(
	current: ConsolidationReviewState,
	action: ConsolidationReviewAction,
) -> Result<Vec<(ConsolidationReviewAction, ConsolidationReviewState)>> {
	let steps = match action {
		ConsolidationReviewAction::Approve => {
			vec![(ConsolidationReviewAction::Approve, ConsolidationReviewState::Approved)]
		},
		ConsolidationReviewAction::Apply => match current {
			ConsolidationReviewState::Proposed => vec![
				(ConsolidationReviewAction::Approve, ConsolidationReviewState::Approved),
				(ConsolidationReviewAction::Apply, ConsolidationReviewState::Applied),
			],
			ConsolidationReviewState::Approved => {
				vec![(ConsolidationReviewAction::Apply, ConsolidationReviewState::Applied)]
			},
			ConsolidationReviewState::Rejected
			| ConsolidationReviewState::Applied
			| ConsolidationReviewState::Archived => {
				vec![(ConsolidationReviewAction::Apply, ConsolidationReviewState::Applied)]
			},
		},
		ConsolidationReviewAction::Discard => {
			vec![(ConsolidationReviewAction::Discard, ConsolidationReviewState::Rejected)]
		},
		ConsolidationReviewAction::Defer => {
			vec![(ConsolidationReviewAction::Defer, ConsolidationReviewState::Archived)]
		},
	};
	let mut state = current;

	for (_, next_state) in &steps {
		state.validate_transition(*next_state).map_err(validation_error)?;

		state = *next_state;
	}

	Ok(steps)
}

pub(super) fn bounded_limit(limit: Option<u32>) -> i64 {
	limit.map(i64::from).unwrap_or(DEFAULT_LIST_LIMIT).clamp(1, MAX_LIST_LIMIT)
}

pub(super) fn to_value<T>(value: &T) -> Result<Value>
where
	T: Serialize,
{
	serde_json::to_value(value).map_err(|err| Error::InvalidRequest {
		message: format!("failed to serialize consolidation contract: {err}"),
	})
}

pub(super) fn terminal_time(
	state: ConsolidationRunState,
	now: OffsetDateTime,
) -> Option<OffsetDateTime> {
	match state {
		ConsolidationRunState::Completed
		| ConsolidationRunState::Failed
		| ConsolidationRunState::Cancelled => Some(now),
		ConsolidationRunState::Pending | ConsolidationRunState::Running => None,
	}
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<()> {
	if value.trim().is_empty() {
		return Err(Error::InvalidRequest { message: format!("{field} must not be empty.") });
	}

	Ok(())
}
