mod audit;
mod document;
mod graph;
mod note;
mod proposal;

use crate::acceptance::knowledge_pages::helpers::KnowledgeSourceIds;
use elf_service::ElfService;

pub(crate) async fn insert_rebuild_sources(service: &ElfService) -> KnowledgeSourceIds {
	let note_id = note::insert_source_note(
		service,
		"knowledge_pages_foundation",
		"Fact: Derived knowledge pages are rebuilt from authoritative source memory and keep citations.",
	)
	.await;
	let event_id = audit::insert_event_audit(service, note_id).await;
	let (doc_id, chunk_id) = document::insert_source_document(service).await;
	let fact_id = graph::insert_relation(service, note_id).await;
	let proposal_id = proposal::insert_applied_proposal(service, note_id).await;

	KnowledgeSourceIds { note_id, event_id, doc_id, chunk_id, fact_id, proposal_id }
}
