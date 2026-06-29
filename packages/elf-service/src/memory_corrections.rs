//! Review-backed memory correction and rollback APIs.

mod service;
mod storage;
#[cfg(test)] mod tests;
mod types;
mod validation;

pub use types::{MemoryCorrectionAction, MemoryCorrectionRequest, MemoryCorrectionResponse};
