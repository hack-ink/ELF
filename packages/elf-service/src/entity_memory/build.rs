pub(in crate::entity_memory) mod core_blocks;
pub(in crate::entity_memory) mod lifecycle;

mod note_items;
mod sort;
mod summary;
mod visibility;

pub(in crate::entity_memory) use self::{
	core_blocks::build_core_block_items, note_items::build_note_items,
	sort::sort_entity_memory_items, summary::summarize_items,
};
