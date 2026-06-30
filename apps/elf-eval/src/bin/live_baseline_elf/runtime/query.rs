use color_eyre::Result;

use crate::{
	AGENT_ID, ElfService, Instant, PROJECT_ID, PayloadLevel, QueryCase, QueryResult, SearchRequest,
	TENANT_ID, Value, env,
};

pub(crate) async fn run_queries(
	service: &ElfService,
	queries: Vec<QueryCase>,
) -> Result<Vec<QueryResult>> {
	let mut out = Vec::with_capacity(queries.len());

	for case in queries {
		out.push(run_single_query(service, case).await?);
	}

	Ok(out)
}

pub(crate) async fn run_single_query(service: &ElfService, case: QueryCase) -> Result<QueryResult> {
	let top_k = env::var("ELF_BASELINE_TOP_K")
		.ok()
		.and_then(|value| value.parse::<u32>().ok())
		.unwrap_or(10);
	let started_at = Instant::now();
	let response = service
		.search_raw(SearchRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: AGENT_ID.to_string(),
			token_id: None,
			payload_level: PayloadLevel::L2,
			read_profile: "private_only".to_string(),
			query: case.query.clone(),
			top_k: Some(top_k),
			candidate_k: Some(top_k.max(20).saturating_mul(4)),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await?;
	let latency_ms = started_at.elapsed().as_secs_f64() * 1_000.0;
	let top = response.items.first();
	let top_text = top.map(|item| item.snippet.clone()).unwrap_or_default();
	let matched_terms = case
		.expected_terms
		.iter()
		.filter(|term| crate::contains_case_insensitive(&top_text, term))
		.cloned()
		.collect::<Vec<_>>();
	let top_key = top.and_then(|item| item.key.clone());
	let expected_docs = crate::expected_docs_for_case(&case);
	let matched_doc = top_key
		.as_deref()
		.and_then(|key| expected_docs.iter().find(|doc| crate::key_for_doc(doc) == key));
	let top_evidence_id = top.and_then(|item| {
		item.source_ref.get("document").and_then(Value::as_str).map(crate::evidence_id_for_doc)
	});
	let matched_evidence_id = matched_doc.map(|doc| crate::evidence_id_for_doc(doc));
	let matched = matched_terms.len() == case.expected_terms.len() || matched_doc.is_some();
	let expected_evidence_ids = if case.expected_evidence_ids.is_empty() {
		vec![crate::evidence_id_for_doc(&case.expected_doc)]
	} else {
		case.expected_evidence_ids.clone()
	};
	let allowed_alternate_evidence_ids = if case.allowed_alternate_evidence_ids.is_empty() {
		case.allowed_alternate_docs.iter().map(|doc| crate::evidence_id_for_doc(doc)).collect()
	} else {
		case.allowed_alternate_evidence_ids.clone()
	};

	Ok(QueryResult {
		id: case.id,
		task: case.task,
		trace_id: response.trace_id,
		query: case.query,
		expected_doc: case.expected_doc,
		allowed_alternate_docs: case.allowed_alternate_docs,
		expected_terms: case.expected_terms,
		expected_evidence_ids,
		allowed_alternate_evidence_ids,
		matched,
		matched_terms,
		top_evidence_id,
		matched_evidence_id,
		top_note_key: top_key,
		top_snippet: top.map(|item| item.snippet.clone()),
		latency_ms,
		returned_count: response.items.len(),
	})
}
