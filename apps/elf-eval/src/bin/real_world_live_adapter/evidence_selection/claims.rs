use crate::evidence_selection::{
	self, BTreeSet, LiveExpectedClaim, LiveMemoryEvolution, LoadedJob, common, serde_json,
};

pub(super) fn answer_claims_impl(
	loaded: &LoadedJob,
	evidence_ids: &[String],
) -> Vec<serde_json::Value> {
	if loaded.job.memory_evolution.is_some() {
		let claims = evidence_selection::temporal_reconciliation_claims(loaded, evidence_ids);

		if !claims.is_empty() {
			return claims;
		}
	}

	evidence_linked_claims(loaded, evidence_ids)
}

pub(super) fn temporal_reconciliation_claims_impl(
	loaded: &LoadedJob,
	evidence_ids: &[String],
) -> Vec<serde_json::Value> {
	let Some(evolution) = &loaded.job.memory_evolution else {
		return Vec::new();
	};
	let selected = evidence_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
	let mut claims = Vec::new();
	let mut claim_ids = BTreeSet::new();

	for expected in &loaded.job.expected_answer.must_include {
		let Some(claim_id) = expected.claim_id() else {
			continue;
		};
		let mut claim_evidence = temporal_claim_evidence(evolution, claim_id, &selected);

		if claim_evidence.is_empty()
			&& let Some(allowed) = loaded.job.expected_answer.evidence_links.get(claim_id)
		{
			claim_evidence = selected_allowed_evidence(allowed, &selected);
		}
		if claim_evidence.is_empty() {
			continue;
		}

		claim_ids.insert(claim_id.to_string());
		claims.push(json_claim(claim_id, expected.text(), claim_evidence));
	}

	if let Some(rationale) = &evolution.update_rationale
		&& rationale.available
		&& !claim_ids.contains(rationale.claim_id.as_str())
	{
		let claim_evidence = rationale
			.evidence_ids
			.iter()
			.filter(|id| selected.contains(id.as_str()))
			.cloned()
			.collect::<Vec<_>>();

		if !claim_evidence.is_empty() {
			let text = expected_claim_text_for_id(loaded, rationale.claim_id.as_str())
				.unwrap_or("The supersession rationale is selected as lifecycle evidence.");

			claims.push(json_claim(rationale.claim_id.as_str(), text, claim_evidence));
		}
	}

	claims
}

fn evidence_linked_claims(loaded: &LoadedJob, evidence_ids: &[String]) -> Vec<serde_json::Value> {
	loaded
		.job
		.expected_answer
		.must_include
		.iter()
		.filter_map(|claim| {
			let claim_id = claim.claim_id()?;
			let allowed =
				evidence_link_ids(loaded.job.expected_answer.evidence_links.get(claim_id)?);
			let produced = evidence_ids
				.iter()
				.filter(|evidence_id| allowed.iter().any(|allowed_id| allowed_id == *evidence_id))
				.cloned()
				.collect::<Vec<_>>();

			if produced.is_empty() {
				return None;
			}

			Some(serde_json::json!({
				"claim_id": claim_id,
				"text": claim.text(),
				"evidence_ids": produced,
				"confidence": "derived_from_live_retrieval"
			}))
		})
		.collect()
}

fn temporal_claim_evidence(
	evolution: &LiveMemoryEvolution,
	claim_id: &str,
	selected: &BTreeSet<&str>,
) -> Vec<String> {
	let mut evidence = Vec::new();

	for conflict in &evolution.conflicts {
		if conflict.claim_id != claim_id {
			continue;
		}

		common::push_if_selected(&mut evidence, conflict.current_evidence_id.as_str(), selected);
		common::push_if_selected(&mut evidence, conflict.historical_evidence_id.as_str(), selected);

		if let Some(rationale_id) = &conflict.resolved_by_evidence_id {
			common::push_if_selected(&mut evidence, rationale_id.as_str(), selected);
		}
	}

	evidence
}

fn selected_allowed_evidence(
	allowed: &serde_json::Value,
	selected: &BTreeSet<&str>,
) -> Vec<String> {
	evidence_link_ids(allowed).into_iter().filter(|id| selected.contains(id.as_str())).collect()
}

fn expected_claim_text_for_id<'a>(loaded: &'a LoadedJob, claim_id: &str) -> Option<&'a str> {
	loaded
		.job
		.expected_answer
		.must_include
		.iter()
		.find(|claim| claim.claim_id() == Some(claim_id))
		.map(LiveExpectedClaim::text)
}

fn json_claim(claim_id: &str, text: &str, evidence_ids: Vec<String>) -> serde_json::Value {
	serde_json::json!({
		"claim_id": claim_id,
		"text": text,
		"evidence_ids": evidence_ids,
		"confidence": "derived_from_live_temporal_reconciliation"
	})
}

fn evidence_link_ids(value: &serde_json::Value) -> Vec<String> {
	if let Some(id) = value.as_str() {
		return vec![id.to_string()];
	}

	value
		.as_array()
		.map(|items| {
			items
				.iter()
				.filter_map(serde_json::Value::as_str)
				.map(ToString::to_string)
				.collect::<Vec<_>>()
		})
		.unwrap_or_default()
}
