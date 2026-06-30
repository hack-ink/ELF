mod admin;
mod read;

pub(super) use self::{
	admin::{
		__path_admin_core_block_attach, __path_admin_core_block_detach,
		__path_admin_core_block_upsert, admin_core_block_attach, admin_core_block_detach,
		admin_core_block_upsert,
	},
	read::{__path_core_blocks_get, __path_entity_memory_get, core_blocks_get, entity_memory_get},
};
