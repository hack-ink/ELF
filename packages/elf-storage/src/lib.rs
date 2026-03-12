#![cfg_attr(test, allow(unused_crate_dependencies))]

//! Storage adapters and row models for ELF persistence backends.

pub mod db;
pub mod doc_outbox;
pub mod docs;
pub mod graph;
pub mod models;
pub mod outbox;
pub mod qdrant;
pub mod queries;
pub mod schema;

mod error;

pub use error::Error;

/// Storage-layer result type.
pub type Result<T, E = Error> = std::result::Result<T, E>;
