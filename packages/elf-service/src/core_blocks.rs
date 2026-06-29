//! Scoped core memory block APIs.

mod persistence;
mod service;
mod types;
mod validation;

pub use types::{
	CoreBlockAttachRequest, CoreBlockAttachResponse, CoreBlockAuditEvent, CoreBlockDetachRequest,
	CoreBlockDetachResponse, CoreBlockItem, CoreBlockRecord, CoreBlockUpsertRequest,
	CoreBlockUpsertResponse, CoreBlocksGetRequest, CoreBlocksResponse,
	ELF_CORE_MEMORY_BLOCKS_SCHEMA_V1,
};
