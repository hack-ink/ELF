//! Event ingestion APIs.

mod audit;
mod materialize;
mod persistence;
mod policy;
mod rejection;
mod service;
mod types;
mod validation;

pub use types::{AddEventRequest, AddEventResponse, AddEventResult, EventMessage};

#[cfg(test)] mod tests;
