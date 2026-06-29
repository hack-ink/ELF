use super::*;

#[derive(Clone, Debug)]
pub(super) struct SourceSnapshot {
	pub(super) kind: KnowledgeSourceKind,
	pub(super) id: Uuid,
	pub(super) status: Option<String>,
	pub(super) updated_at: Option<OffsetDateTime>,
	pub(super) content_hash: Option<String>,
	pub(super) snapshot: Value,
	pub(super) citation_metadata: Value,
	pub(super) line: String,
}

#[derive(Clone, Debug)]
pub(super) struct DraftSection {
	pub(super) section_id: Uuid,
	pub(super) section_key: String,
	pub(super) heading: String,
	pub(super) role: String,
	pub(super) content: String,
	pub(super) ordinal: i32,
	pub(super) source_indexes: Vec<usize>,
	pub(super) unsupported_reason: Option<String>,
	pub(super) content_hash: String,
	pub(super) citations: Value,
}

#[derive(Clone, Debug)]
pub(super) struct LintDraft {
	pub(super) section_id: Option<Uuid>,
	pub(super) finding_type: String,
	pub(super) severity: String,
	pub(super) source_kind: Option<KnowledgeSourceKind>,
	pub(super) source_id: Option<Uuid>,
	pub(super) message: String,
	pub(super) details: Value,
}

#[derive(Clone, Debug)]
pub(super) struct SourceIds {
	pub(super) doc_ids: Vec<Uuid>,
	pub(super) doc_chunk_ids: Vec<Uuid>,
	pub(super) note_ids: Vec<Uuid>,
	pub(super) event_ids: Vec<Uuid>,
	pub(super) relation_ids: Vec<Uuid>,
	pub(super) proposal_ids: Vec<Uuid>,
}
impl SourceIds {
	pub(super) fn from_request(req: &KnowledgePageRebuildRequest) -> Result<Self> {
		let ids = Self {
			doc_ids: sorted_unique(&req.doc_ids),
			doc_chunk_ids: sorted_unique(&req.doc_chunk_ids),
			note_ids: sorted_unique(&req.note_ids),
			event_ids: sorted_unique(&req.event_ids),
			relation_ids: sorted_unique(&req.relation_ids),
			proposal_ids: sorted_unique(&req.proposal_ids),
		};

		ids.validate_non_empty()?;

		Ok(ids)
	}

	pub(super) fn from_source_refs(source_refs: &[KnowledgePageSourceRef]) -> Result<Self> {
		let mut doc_ids = Vec::new();
		let mut doc_chunk_ids = Vec::new();
		let mut note_ids = Vec::new();
		let mut event_ids = Vec::new();
		let mut relation_ids = Vec::new();
		let mut proposal_ids = Vec::new();

		for source_ref in source_refs {
			match KnowledgeSourceKind::parse(source_ref.source_kind.as_str()) {
				Some(KnowledgeSourceKind::Doc) => doc_ids.push(source_ref.source_id),
				Some(KnowledgeSourceKind::DocChunk) => doc_chunk_ids.push(source_ref.source_id),
				Some(KnowledgeSourceKind::Note) => note_ids.push(source_ref.source_id),
				Some(KnowledgeSourceKind::Event) => event_ids.push(source_ref.source_id),
				Some(KnowledgeSourceKind::Relation) => relation_ids.push(source_ref.source_id),
				Some(KnowledgeSourceKind::Proposal) => proposal_ids.push(source_ref.source_id),
				None => {
					return Err(Error::InvalidRequest {
						message: "stored knowledge page source kind is invalid".to_string(),
					});
				},
			}
		}

		Ok(Self {
			doc_ids: sorted_unique(&doc_ids),
			doc_chunk_ids: sorted_unique(&doc_chunk_ids),
			note_ids: sorted_unique(&note_ids),
			event_ids: sorted_unique(&event_ids),
			relation_ids: sorted_unique(&relation_ids),
			proposal_ids: sorted_unique(&proposal_ids),
		})
	}

	pub(super) fn validate_non_empty(&self) -> Result<()> {
		if self.doc_ids.is_empty()
			&& self.doc_chunk_ids.is_empty()
			&& self.note_ids.is_empty()
			&& self.event_ids.is_empty()
			&& self.relation_ids.is_empty()
			&& self.proposal_ids.is_empty()
		{
			return Err(Error::InvalidRequest {
				message: "at least one source id is required for a knowledge page rebuild"
					.to_string(),
			});
		}

		Ok(())
	}

	pub(super) fn require_counts(
		&self,
		docs: usize,
		doc_chunks: usize,
		notes: usize,
		events: usize,
		relations: usize,
		proposals: usize,
	) -> Result<()> {
		if docs != self.doc_ids.len()
			|| doc_chunks != self.doc_chunk_ids.len()
			|| notes != self.note_ids.len()
			|| events != self.event_ids.len()
			|| relations != self.relation_ids.len()
			|| proposals != self.proposal_ids.len()
		{
			return Err(Error::InvalidRequest {
				message:
					"all requested knowledge page sources must exist, source rows must be active and readable, and proposals must be applied"
						.to_string(),
			});
		}

		Ok(())
	}
}

pub(super) struct WatchRebuildOutcome {
	pub(super) item: KnowledgePageWatchRebuildItem,
	pub(super) candidates: Vec<KnowledgeDeltaMemoryCandidate>,
}
