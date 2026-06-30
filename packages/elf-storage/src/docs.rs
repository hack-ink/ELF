//! Document persistence queries.

mod chunks;
mod documents;
mod embeddings;

pub use self::{
	chunks::{get_doc_chunk, insert_doc_chunk, list_doc_chunks},
	documents::{get_doc_document, insert_doc_document, mark_doc_deleted, normalize_source_ref},
	embeddings::insert_doc_chunk_embedding,
};
