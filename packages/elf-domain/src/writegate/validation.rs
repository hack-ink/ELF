use super::*;

/// Validates note content and metadata against ELF write-gate rules.
pub fn writegate(note: &NoteInput, cfg: &Config) -> Result<(), RejectCode> {
	if note.text.trim().is_empty() {
		return Err(RejectCode::RejectEmpty);
	}
	if !english_gate::is_english_natural_language(note.text.as_str()) {
		return Err(RejectCode::RejectNonEnglish);
	}
	if note.text.chars().count() as u32 > cfg.memory.max_note_chars {
		return Err(RejectCode::RejectTooLong);
	}
	if !is_allowed_type(&note.note_type) {
		return Err(RejectCode::RejectInvalidType);
	}
	if !cfg.scopes.allowed.iter().any(|scope| scope == &note.scope) {
		return Err(RejectCode::RejectScopeDenied);
	}
	if !scope_write_allowed(cfg, &note.scope) {
		return Err(RejectCode::RejectScopeDenied);
	}
	if contains_secrets(&note.text) {
		return Err(RejectCode::RejectSecret);
	}

	Ok(())
}

fn scope_write_allowed(cfg: &Config, scope: &str) -> bool {
	match scope {
		"agent_private" => cfg.scopes.write_allowed.agent_private,
		"project_shared" => cfg.scopes.write_allowed.project_shared,
		"org_shared" => cfg.scopes.write_allowed.org_shared,
		_ => false,
	}
}

fn is_allowed_type(note_type: &str) -> bool {
	matches!(note_type, "preference" | "constraint" | "decision" | "profile" | "fact" | "plan")
}
