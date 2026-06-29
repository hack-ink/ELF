use super::*;

impl ElfService {
	pub(super) async fn recall_graph_layer(
		&self,
		req: &RecallDebugPanelRequest,
		limit: u32,
	) -> Result<RecallDebugLayer> {
		let Some(subject) = req.graph_subject.clone() else {
			return Ok(not_requested_layer(
				"graph_facts",
				"Supply graph_subject to show graph fact candidates and temporal status.",
			));
		};
		let response = self
			.graph_report(GraphReportRequest {
				tenant_id: req.tenant_id.clone(),
				project_id: req.project_id.clone(),
				agent_id: req.agent_id.clone(),
				read_profile: req.read_profile.clone(),
				subject,
				predicate: req.graph_predicate.clone(),
				scopes: None,
				as_of: None,
				limit: Some(limit),
				explain: Some(true),
			})
			.await?;
		let subject_anchor = response.subject.canonical.clone();
		let replay_command = graph_replay_command(&subject_anchor, req.graph_predicate.as_ref());
		let rows = response
			.facts
			.into_iter()
			.enumerate()
			.map(|(index, fact)| RecallDebugRow {
				layer: "graph_facts".to_string(),
				item_ref: serde_json::json!({
					"fact_id": fact.fact_id,
					"subject": subject_anchor,
					"predicate": fact.predicate,
					"object": fact.object,
				}),
				selection_state: "available".to_string(),
				authority_layer: "graph_fact".to_string(),
				freshness_state: graph_temporal_status(fact.temporal_status),
				source_refs: serde_json::json!({
					"evidence_note_ids": fact.evidence_note_ids,
					"supersedes_fact_ids": fact.supersedes_fact_ids,
					"superseded_by_fact_ids": fact.superseded_by_fact_ids,
				}),
				score: None,
				rank: Some(index as u32 + 1),
				rationale: Some("graph_report returned source-backed fact".to_string()),
				stage_reason: Some(fact.status_markers.join(",")),
				replay_command: Some(replay_command.clone()),
				evidence_class: "pass".to_string(),
				debug_artifacts: serde_json::json!({
					"scope": fact.scope,
					"actor": fact.actor,
					"valid_from": fact.valid_from,
					"valid_to": fact.valid_to,
					"status_markers": fact.status_markers,
				}),
			})
			.collect();

		Ok(layer_from_rows_with_artifacts(
			"graph_facts",
			"pass",
			Some(subject_anchor),
			"Graph facts from source-backed graph report.",
			rows,
			serde_json::json!({}),
		))
	}
}
