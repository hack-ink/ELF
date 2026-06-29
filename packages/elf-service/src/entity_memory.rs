//! Entity-scoped memory authority readback.

mod build;
mod service;
mod storage;
#[cfg(test)] mod tests;
mod types;
mod validation;

/// Entity memory view response schema identifier.
pub const ELF_ENTITY_MEMORY_VIEW_SCHEMA_V1: &str = "elf.entity_memory_view/v1";

const TOP_OF_MIND_IMPORTANCE_THRESHOLD: f32 = 0.8;

pub use types::{
	EntityMemoryEntity, EntityMemoryItem, EntityMemoryRelation, EntityMemorySummary,
	EntityMemoryViewRequest, EntityMemoryViewResponse,
};
