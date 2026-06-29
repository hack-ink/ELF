//! Review-backed memory correction and rollback APIs.

mod service;
mod storage;
mod types;
mod validation;

pub use types::{MemoryCorrectionAction, MemoryCorrectionRequest, MemoryCorrectionResponse};

#[cfg(test)] mod tests;
