mod add_note_no_llm;
mod chunk_search;
mod chunking;
#[path = "suite/config.rs"] mod config;
mod consolidation;
mod docs_extension_v1;
mod english_only_boundary;
mod evidence_binding;
mod graph_ingestion;
mod idempotency;
mod knowledge_pages;
mod memory_history;
mod outbox_eventual_consistency;
#[path = "suite/providers.rs"] mod providers;
mod rebuild_qdrant;
#[path = "suite/runtime.rs"] mod runtime;
mod sot_vectors;
mod structured_field_retrieval;
mod trace_admin_observability;
mod work_journal;

pub(crate) use self::{
	config::{dummy_embedding_provider, test_config, test_qdrant_url},
	providers::{SpyEmbedding, SpyExtractor, StubEmbedding, StubRerank},
	runtime::{build_service, reset_db, reset_qdrant_collection, test_db},
};
