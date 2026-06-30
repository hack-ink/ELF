mod ingest;
mod publish;
mod read;
mod write;

pub(super) use self::{
	ingest::{__path_notes_ingest, notes_ingest},
	publish::{__path_notes_publish, __path_notes_unpublish, notes_publish, notes_unpublish},
	read::{__path_notes_get, __path_notes_list, notes_get, notes_list},
	write::{__path_notes_delete, __path_notes_patch, notes_delete, notes_patch},
};
