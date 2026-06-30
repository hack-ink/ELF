mod corrections;
mod read;

pub(super) use self::{
	corrections::{__path_admin_note_correction_apply, admin_note_correction_apply},
	read::{
		__path_admin_note_history_get, __path_admin_note_provenance_get, admin_note_history_get,
		admin_note_provenance_get,
	},
};
