use color_eyre::Result;
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

pub(super) fn sql_uuid(id: &Uuid) -> String {
	format!("'{}'", id)
}

pub(super) fn sql_opt_uuid(id: &Option<Uuid>) -> String {
	id.map(|value| format!("'{}'", value)).unwrap_or_else(|| "NULL".to_string())
}

pub(super) fn sql_text(value: &str) -> String {
	format!("'{}'", value.replace('\'', "''"))
}

pub(super) fn sql_jsonb(value: &Value) -> Result<String> {
	let raw = serde_json::to_string(value)?;

	Ok(format!("'{}'::jsonb", raw.replace('\'', "''")))
}

pub(super) fn sql_f32(value: f32) -> String {
	format!("{value}")
}

pub(super) fn sql_timestamptz(value: &OffsetDateTime) -> Result<String> {
	let raw = value.format(&Rfc3339)?;

	Ok(format!("'{}'::timestamptz", raw.replace('\'', "''")))
}

pub(super) fn sql_opt_timestamptz(value: &Option<OffsetDateTime>) -> Result<String> {
	match value {
		Some(ts) => sql_timestamptz(ts),
		None => Ok("NULL".to_string()),
	}
}
