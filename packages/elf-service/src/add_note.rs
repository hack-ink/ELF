//! Direct note ingestion APIs.

mod audit;
mod materialize;
mod persistence;
mod policy;
mod rejection;
mod service;
mod types;
mod validation;

pub use types::{AddNoteInput, AddNoteRequest, AddNoteResponse, AddNoteResult};

#[cfg(test)] mod tests;
