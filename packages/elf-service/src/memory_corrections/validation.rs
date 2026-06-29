use serde_json::{Map, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, Result, memory_corrections::types::MemoryCorrectionAction};
use elf_config::Scopes;
use elf_storage::models::MemoryNote;

pub(super) fn validate_correction_request(
	tenant_id: &str,
	project_id: &str,
	actor_agent_id: &str,
	reason: &str,
	source_ref: &Value,
) -> Result<()> {
	if tenant_id.is_empty() || project_id.is_empty() || actor_agent_id.is_empty() {
		return Err(Error::InvalidRequest {
			message: "tenant_id, project_id, and actor_agent_id are required.".to_string(),
		});
	}
	if reason.is_empty() {
		return Err(Error::InvalidRequest { message: "reason must not be empty.".to_string() });
	}
	if !is_non_empty_object(source_ref) {
		return Err(Error::InvalidRequest {
			message: "source_ref must be a non-empty JSON object.".to_string(),
		});
	}

	Ok(())
}

pub(super) fn validate_write_scope(note: &MemoryNote, scopes: &Scopes) -> Result<()> {
	if !scopes.allowed.iter().any(|scope| scope == &note.scope) {
		return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
	}

	let write_allowed = match note.scope.as_str() {
		"agent_private" => scopes.write_allowed.agent_private,
		"project_shared" => scopes.write_allowed.project_shared,
		"org_shared" => scopes.write_allowed.org_shared,
		_ => false,
	};

	if write_allowed {
		Ok(())
	} else {
		Err(Error::ScopeDenied { message: "Scope is not writable.".to_string() })
	}
}

pub(super) fn apply_restore_snapshot(
	note: &mut MemoryNote,
	snapshot: &Value,
	now: OffsetDateTime,
) -> Result<()> {
	let status = required_string(snapshot, "status")?;

	if status != "active" {
		return Err(Error::InvalidRequest {
			message: "Restore snapshot must represent an active memory.".to_string(),
		});
	}

	note.scope = required_string(snapshot, "scope")?;
	note.r#type = required_string(snapshot, "type")?;
	note.key = optional_string(snapshot, "key")?;
	note.text = required_string(snapshot, "text")?;
	note.importance = required_f32(snapshot, "importance")?;
	note.confidence = required_f32(snapshot, "confidence")?;
	note.status = status;
	note.updated_at = now;
	note.expires_at = optional_offset_datetime(snapshot, "expires_at")?;

	Ok(())
}

pub(super) fn correction_source_ref_for(
	action: MemoryCorrectionAction,
	prior_snapshot: &Value,
	correction_source_ref: &Value,
	reason: &str,
	actor_agent_id: &str,
	now: OffsetDateTime,
	restore_version_id: Option<Uuid>,
) -> Value {
	serde_json::json!({
		"schema": "elf.memory_correction/v1",
		"action": action.as_str(),
		"reason": reason,
		"actor_agent_id": actor_agent_id,
		"ts": now,
		"restore_version_id": restore_version_id,
		"prior_source_ref": prior_snapshot.get("source_ref").cloned().unwrap_or_else(empty_object),
		"prior_snapshot": prior_snapshot,
		"correction_source_ref": correction_source_ref,
	})
}

fn is_non_empty_object(value: &Value) -> bool {
	matches!(value, Value::Object(map) if !map.is_empty())
}

fn required_string(snapshot: &Value, field: &'static str) -> Result<String> {
	snapshot
		.get(field)
		.and_then(Value::as_str)
		.map(str::to_string)
		.filter(|value| !value.trim().is_empty())
		.ok_or_else(|| Error::InvalidRequest {
			message: format!("Restore snapshot field {field} must be a non-empty string."),
		})
}

fn optional_string(snapshot: &Value, field: &'static str) -> Result<Option<String>> {
	match snapshot.get(field) {
		None | Some(Value::Null) => Ok(None),
		Some(Value::String(value)) => Ok(Some(value.clone())),
		_ => Err(Error::InvalidRequest {
			message: format!("Restore snapshot field {field} must be a string or null."),
		}),
	}
}

fn required_f32(snapshot: &Value, field: &'static str) -> Result<f32> {
	let Some(value) = snapshot.get(field).and_then(Value::as_f64) else {
		return Err(Error::InvalidRequest {
			message: format!("Restore snapshot field {field} must be a number."),
		});
	};

	if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
		return Err(Error::InvalidRequest {
			message: format!("Restore snapshot field {field} is out of range."),
		});
	}

	Ok(value as f32)
}

fn optional_offset_datetime(
	snapshot: &Value,
	field: &'static str,
) -> Result<Option<OffsetDateTime>> {
	let Some(value) = snapshot.get(field) else {
		return Ok(None);
	};

	serde_json::from_value(value.clone()).map_err(|err| Error::InvalidRequest {
		message: format!("Restore snapshot field {field} is not a valid timestamp: {err}."),
	})
}

fn empty_object() -> Value {
	Value::Object(Map::new())
}
