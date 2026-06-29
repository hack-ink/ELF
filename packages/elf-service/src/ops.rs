use serde::{Deserialize, Serialize};

/// Note operation emitted by service mutations.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NoteOp {
	/// A new note was inserted.
	Add,
	/// An existing note was updated.
	Update,
	/// No persisted change was required.
	None,
	/// A note was deleted.
	Delete,
	/// The request was rejected before persistence.
	Rejected,
}
