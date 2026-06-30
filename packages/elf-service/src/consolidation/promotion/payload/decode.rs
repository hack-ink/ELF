use serde_json::Value;

use crate::{Error, Result, consolidation::types::PromotedMemoryPayload};
use elf_storage::models::ConsolidationProposal;

pub(in crate::consolidation) fn decode_promoted_memory_payload(
	proposal: &ConsolidationProposal,
) -> Result<PromotedMemoryPayload> {
	let payload: PromotedMemoryPayload = serde_json::from_value(proposal.proposed_payload.clone())
		.map_err(|err| Error::InvalidRequest {
			message: format!("proposed_payload is not a memory note payload: {err}"),
		})?;

	if !matches!(payload.source_ref, Value::Object(_)) {
		return Err(Error::InvalidRequest {
			message: "proposed_payload.source_ref must be a JSON object when provided.".to_string(),
		});
	}
	if payload.importance.is_some_and(invalid_score)
		|| payload.confidence.is_some_and(invalid_score)
	{
		return Err(Error::InvalidRequest {
			message: "proposed memory scores must be finite values in 0.0..=1.0.".to_string(),
		});
	}

	Ok(payload)
}

fn invalid_score(score: f32) -> bool {
	!score.is_finite() || !(0.0..=1.0).contains(&score)
}
