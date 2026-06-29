//! Database row models shared across storage modules.

mod consolidation;
mod docs;
mod graph;
mod knowledge;
mod notes;
mod outbox;
mod work_journal;

pub use self::{
	consolidation::{
		ConsolidationProposal, ConsolidationProposalReviewEvent, ConsolidationRun,
		ConsolidationRunJob,
	},
	docs::{DocChunk, DocChunkEmbedding, DocDocument, DocIndexingOutboxEntry},
	graph::{
		GraphEntity, GraphEntityAlias, GraphFact, GraphFactEvidence, GraphFactSupersession,
		GraphPredicate, GraphPredicateAlias,
	},
	knowledge::{
		KnowledgePage, KnowledgePageLintFinding, KnowledgePageSection, KnowledgePageSourceRef,
	},
	notes::{MemoryNote, MemoryNoteChunk, NoteChunkEmbedding, NoteEmbedding},
	outbox::{IndexingOutboxEntry, TraceOutboxJob},
	work_journal::WorkJournalEntry,
};
