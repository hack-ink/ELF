use super::*;

impl ElfService {
	pub(super) async fn recall_dreaming_layer(
		&self,
		req: &RecallDebugPanelRequest,
		include_dreaming: bool,
		limit: u32,
	) -> Result<RecallDebugLayer> {
		if !include_dreaming {
			return Ok(not_requested_layer(
				"dreaming_proposals",
				"Set include_dreaming=true to show reviewable Dreaming proposals.",
			));
		}

		let response = self
			.dreaming_review_queue(DreamingReviewQueueRequest {
				tenant_id: req.tenant_id.clone(),
				project_id: req.project_id.clone(),
				run_id: None,
				review_state: None,
				limit: Some(limit),
			})
			.await?;
		let rows = response
			.items
			.into_iter()
			.enumerate()
			.map(|(index, item)| RecallDebugRow {
				layer: "dreaming_proposals".to_string(),
				item_ref: serde_json::json!({
					"proposal_id": item.proposal_id,
					"run_id": item.run_id,
					"queue_variant": item.queue_variant,
					"target_ref": item.target_ref,
				}),
				selection_state: "reviewable".to_string(),
				authority_layer: "reviewable_dreaming_proposal".to_string(),
				freshness_state: item.review_state.clone(),
				source_refs: serde_json::json!({
					"source_refs": item.source_refs,
					"source_snapshot": item.source_snapshot,
					"affected_refs": item.affected_refs,
				}),
				score: Some(item.confidence),
				rank: Some(index as u32 + 1),
				rationale: Some(item.policy.reason.clone()),
				stage_reason: Some(format!(
					"review_state={}, auto_apply_allowed={}",
					item.review_state, item.policy.auto_apply_allowed
				)),
				replay_command: Some("elf_dreaming_review_queue limit=<n>".to_string()),
				evidence_class: "pass".to_string(),
				debug_artifacts: serde_json::json!({
					"policy": item.policy,
					"unsupported_claim_flags": item.unsupported_claim_flags,
					"contradiction_markers": item.contradiction_markers,
					"staleness_markers": item.staleness_markers,
					"diff": item.diff,
					"review_audit": item.review_audit,
				}),
			})
			.collect();

		Ok(layer_from_rows_with_artifacts(
			"dreaming_proposals",
			"pass",
			None,
			"Dreaming review queue proposals available for reviewer action.",
			rows,
			serde_json::json!({}),
		))
	}
}
