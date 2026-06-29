//! Derived knowledge page persistence and source-snapshot queries.

mod queries;
mod sources;
mod types;
mod writes;

pub use self::{
	queries::{
		get_knowledge_page, get_knowledge_page_by_key, list_knowledge_page_lint_findings,
		list_knowledge_page_sections, list_knowledge_page_source_refs,
		list_knowledge_page_source_refs_for_pages, list_knowledge_pages,
		list_knowledge_pages_for_sources, search_knowledge_page_sections,
	},
	sources::{
		fetch_knowledge_doc_chunk_sources, fetch_knowledge_doc_sources,
		fetch_knowledge_event_sources, fetch_knowledge_note_sources,
		fetch_knowledge_proposal_sources, fetch_knowledge_relation_sources,
	},
	types::{
		KnowledgeDocChunkSource, KnowledgeDocSource, KnowledgeEventSource, KnowledgeNoteSource,
		KnowledgePageLintFindingInsert, KnowledgePageSearchRow, KnowledgePageSectionInsert,
		KnowledgePageSourceRefInsert, KnowledgePageUpsert, KnowledgeProposalSource,
		KnowledgeRelationSource, KnowledgeRelationSourcesFetch,
	},
	writes::{
		delete_knowledge_page_children, delete_knowledge_page_lint_findings,
		insert_knowledge_page_lint_finding, insert_knowledge_page_section,
		insert_knowledge_page_source_ref, upsert_knowledge_page,
	},
};
