use crate::knowledge::watch::{
	BTreeSet, Error, KnowledgePage, KnowledgePageChangedSource, KnowledgePageKind,
	KnowledgePageRebuildRequest, KnowledgePageSourceRef, Result, SourceIds, Uuid, Value,
	empty_object,
};

pub(in crate::knowledge) fn normalized_changed_sources(
	changed_sources: &[KnowledgePageChangedSource],
) -> Result<Vec<KnowledgePageChangedSource>> {
	if changed_sources.is_empty() {
		return Err(Error::InvalidRequest {
			message: "changed_sources must not be empty.".to_string(),
		});
	}

	let mut seen = BTreeSet::new();
	let mut out = Vec::new();

	for source in changed_sources {
		if seen.insert((source.source_kind.as_str().to_string(), source.source_id)) {
			out.push(source.clone());
		}
	}

	Ok(out)
}

pub(in crate::knowledge) fn changed_source_arrays(
	changed_sources: &[KnowledgePageChangedSource],
) -> (Vec<String>, Vec<Uuid>) {
	changed_sources
		.iter()
		.map(|source| (source.source_kind.as_str().to_string(), source.source_id))
		.unzip()
}

pub(in crate::knowledge) fn rebuild_request_from_page(
	agent_id: &str,
	page: &KnowledgePage,
	source_refs: &[KnowledgePageSourceRef],
) -> Result<KnowledgePageRebuildRequest> {
	let ids = SourceIds::from_source_refs(source_refs)?;
	let page_kind = KnowledgePageKind::parse(page.page_kind.as_str()).ok_or_else(|| {
		Error::InvalidRequest { message: "stored knowledge page kind is invalid".to_string() }
	})?;
	let provider_metadata = page
		.rebuild_metadata
		.get("provider_metadata")
		.filter(|metadata| matches!(metadata, Value::Object(_)))
		.cloned()
		.unwrap_or_else(empty_object);

	Ok(KnowledgePageRebuildRequest {
		tenant_id: page.tenant_id.clone(),
		project_id: page.project_id.clone(),
		agent_id: agent_id.to_string(),
		page_kind,
		page_key: page.page_key.clone(),
		title: Some(page.title.clone()),
		doc_ids: ids.doc_ids,
		doc_chunk_ids: ids.doc_chunk_ids,
		note_ids: ids.note_ids,
		event_ids: ids.event_ids,
		relation_ids: ids.relation_ids,
		proposal_ids: ids.proposal_ids,
		provider_metadata,
	})
}

pub(in crate::knowledge) fn default_generate_memory_candidates() -> bool {
	true
}
