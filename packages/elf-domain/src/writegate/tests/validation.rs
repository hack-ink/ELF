use crate::writegate::{self, NoteInput, RejectCode, tests::config};

#[test]
fn rejects_long_text() {
	let cfg = config::config();
	let note = NoteInput {
		note_type: "fact".to_string(),
		scope: "agent_private".to_string(),
		text: "12345678901".to_string(),
	};

	assert_eq!(writegate::writegate(&note, &cfg), Err(RejectCode::RejectTooLong));
}

#[test]
fn rejects_invalid_type() {
	let cfg = config::config();
	let note = NoteInput {
		note_type: "other".to_string(),
		scope: "agent_private".to_string(),
		text: "hello".to_string(),
	};

	assert_eq!(writegate::writegate(&note, &cfg), Err(RejectCode::RejectInvalidType));
}

#[test]
fn detects_secret_patterns() {
	assert!(writegate::contains_secrets("password: hunter2"));
}
