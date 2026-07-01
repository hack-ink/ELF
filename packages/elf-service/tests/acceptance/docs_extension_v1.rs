mod excerpts;
mod helpers;
mod indexing;
mod l0_search;
mod lifecycle;
mod search_filters;
mod validation_rejections;

pub(crate) use helpers::{
	DocsContext, TEST_CONTENT, assert_doc_excerpt, assert_doc_get, assert_docs_search_l0,
	cleanup_docs_filter_fixture, create_docs_search_filter_fixture, fetch_first_doc_chunk_id,
	fetch_first_doc_chunk_point, payload_string, put_test_doc, put_test_doc_with,
	search_doc_ids_with_filters, setup_docs_context, spawn_doc_worker, trajectory_stage_stats,
	verify_docs_qdrant_filter_indexes, wait_for_doc_outbox_done, wait_for_note_outbox_done,
};
