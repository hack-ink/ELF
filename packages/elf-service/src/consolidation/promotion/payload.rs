mod decode;
mod refs;
mod scope;
mod validate;

pub(in crate::consolidation) use self::{
	decode::decode_promoted_memory_payload,
	refs::{promoted_memory_target_ref, promotion_source_ref, target_note_id},
	scope::{normalized_optional_string, promoted_memory_project_id, promoted_memory_scope},
	validate::validate_promoted_memory_payload,
};

#[cfg(test)] mod tests;
