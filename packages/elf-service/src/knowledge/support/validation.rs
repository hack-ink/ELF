use crate::knowledge::support::{Error, Map, Result, Value};

pub(in crate::knowledge) fn validate_context(
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
) -> Result<()> {
	validate_non_empty("tenant_id", tenant_id)?;
	validate_non_empty("project_id", project_id)?;

	validate_non_empty("agent_id", agent_id)
}

pub(in crate::knowledge) fn validate_non_empty(field: &'static str, value: &str) -> Result<()> {
	if value.trim().is_empty() {
		return Err(Error::InvalidRequest { message: format!("{field} must not be empty.") });
	}

	Ok(())
}

pub(in crate::knowledge) fn validate_object(field: &str, value: &Value) -> Result<()> {
	if matches!(value, Value::Object(_)) {
		Ok(())
	} else {
		Err(Error::InvalidRequest { message: format!("{field} must be a JSON object.") })
	}
}

pub(in crate::knowledge) fn empty_object() -> Value {
	Value::Object(Map::new())
}
