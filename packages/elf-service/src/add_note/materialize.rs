mod add;
mod none;
mod structured_materialization;
mod update;

pub(super) use self::{
	add::handle_add_note_add, none::handle_add_note_none, update::handle_add_note_update,
};
