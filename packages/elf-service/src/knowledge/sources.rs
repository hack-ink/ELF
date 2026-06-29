use crate::knowledge::{
	self, BTreeSet, HashMap, HashSet, KnowledgeDocChunkSource, KnowledgeDocSource,
	KnowledgeEventSource, KnowledgeNoteSource, KnowledgePageSourceRef, KnowledgeProposalSource,
	KnowledgeRelationSource, SourceSnapshot, Uuid, Value, access, doc_chunk_source_snapshot,
	doc_source_snapshot, event_source_snapshot, note_source_snapshot, proposal_source_snapshot,
	relation_source_snapshot, source_sort_key,
};

pub(super) fn source_snapshots(
	docs: Vec<KnowledgeDocSource>,
	doc_chunks: Vec<KnowledgeDocChunkSource>,
	notes: Vec<KnowledgeNoteSource>,
	events: Vec<KnowledgeEventSource>,
	relations: Vec<KnowledgeRelationSource>,
	proposals: Vec<KnowledgeProposalSource>,
) -> Vec<SourceSnapshot> {
	let mut sources = Vec::new();

	sources.extend(docs.into_iter().map(doc_source_snapshot));
	sources.extend(doc_chunks.into_iter().map(doc_chunk_source_snapshot));
	sources.extend(notes.into_iter().map(note_source_snapshot));
	sources.extend(events.into_iter().map(event_source_snapshot));
	sources.extend(relations.into_iter().map(relation_source_snapshot));
	sources.extend(proposals.into_iter().map(proposal_source_snapshot));
	sources.sort_by_key(source_sort_key);

	sources
}

pub(super) fn source_refs_by_section(
	source_refs: &[KnowledgePageSourceRef],
) -> HashMap<Uuid, Vec<KnowledgePageSourceRef>> {
	let mut by_section = HashMap::<Uuid, Vec<KnowledgePageSourceRef>>::new();

	for source_ref in source_refs {
		let Some(section_id) = source_ref.section_id else {
			continue;
		};

		by_section.entry(section_id).or_default().push(clone_source_ref(source_ref));
	}

	by_section
}

pub(super) fn recallable_source_refs(
	source_refs: &[KnowledgePageSourceRef],
	current_source_keys: &BTreeSet<String>,
) -> bool {
	!source_refs.is_empty()
		&& source_refs.iter().all(|source_ref| {
			current_source_keys.contains(&knowledge::current_key(
				source_ref.source_kind.as_str(),
				source_ref.source_id,
			)) && recallable_source_ref(source_ref)
		})
}

pub(super) fn source_row_read_allowed(
	owner_agent_id: &str,
	scope: &str,
	requester_agent_id: Option<&str>,
	allowed_scopes: &[String],
	shared_grants: &HashSet<access::SharedSpaceGrantKey>,
) -> bool {
	if !allowed_scopes.iter().any(|allowed_scope| allowed_scope == scope) {
		return false;
	}

	let Some(requester_agent_id) = requester_agent_id else {
		return true;
	};

	if scope == "agent_private" {
		return owner_agent_id == requester_agent_id;
	}
	if !matches!(scope, "project_shared" | "org_shared") {
		return false;
	}
	if owner_agent_id == requester_agent_id {
		return true;
	}

	shared_grants.contains(&access::SharedSpaceGrantKey {
		scope: scope.to_string(),
		space_owner_agent_id: owner_agent_id.to_string(),
	})
}

pub(super) fn recallable_source_ref(source_ref: &KnowledgePageSourceRef) -> bool {
	let Some(status) = source_ref.source_status.as_deref().map(str::trim) else {
		return false;
	};

	if !matches!(status, "active" | "remember" | "update" | "current" | "historical" | "applied") {
		return false;
	}

	!has_non_recallable_span(&source_ref.source_snapshot)
}

pub(super) fn has_non_recallable_span(source_snapshot: &Value) -> bool {
	match source_snapshot {
		Value::Object(object) =>
			policy_spans_are_non_recallable(object.get("policy_spans"))
				|| object.get("source_span").is_some_and(span_is_non_recallable)
				|| source_spans_are_non_recallable(object.get("source_spans"))
				|| object.values().any(has_non_recallable_span),
		Value::Array(items) => items.iter().any(has_non_recallable_span),
		_ => false,
	}
}

pub(super) fn policy_spans_are_non_recallable(policy_spans: Option<&Value>) -> bool {
	match policy_spans {
		Some(Value::Array(spans)) => !spans.is_empty(),
		Some(Value::Null) | None => false,
		Some(_) => true,
	}
}

pub(super) fn source_spans_are_non_recallable(source_spans: Option<&Value>) -> bool {
	match source_spans {
		Some(Value::Array(spans)) => spans.iter().any(span_is_non_recallable),
		Some(Value::Null) | None => false,
		Some(_) => true,
	}
}

pub(super) fn span_is_non_recallable(span: &Value) -> bool {
	!matches!(span.get("status").and_then(Value::as_str), Some("captured"))
}

pub(super) fn cloned_source_refs(
	source_refs: Option<&Vec<KnowledgePageSourceRef>>,
) -> Vec<KnowledgePageSourceRef> {
	source_refs.map(|refs| refs.iter().map(clone_source_ref).collect()).unwrap_or_default()
}

pub(super) fn clone_source_ref(source_ref: &KnowledgePageSourceRef) -> KnowledgePageSourceRef {
	KnowledgePageSourceRef {
		ref_id: source_ref.ref_id,
		page_id: source_ref.page_id,
		section_id: source_ref.section_id,
		source_kind: source_ref.source_kind.clone(),
		source_id: source_ref.source_id,
		source_status: source_ref.source_status.clone(),
		source_updated_at: source_ref.source_updated_at,
		source_content_hash: source_ref.source_content_hash.clone(),
		source_snapshot: source_ref.source_snapshot.clone(),
		citation_metadata: source_ref.citation_metadata.clone(),
		created_at: source_ref.created_at,
	}
}
