use crate::work_journal::validation::{self, Error, MAX_SIDE_LIST_ITEMS, Result, Value};

pub(in crate::work_journal) fn validate_source_refs(source_refs: &[Value]) -> Result<Value> {
	if source_refs.is_empty() {
		return Err(Error::InvalidRequest {
			message: "source_refs must be non-empty.".to_string(),
		});
	}
	if source_refs.len() > MAX_SIDE_LIST_ITEMS {
		return Err(Error::InvalidRequest {
			message: "source_refs has too many items.".to_string(),
		});
	}

	for (index, source_ref) in source_refs.iter().enumerate() {
		match source_ref {
			Value::Object(map) if !map.is_empty() => {},
			_ => {
				return Err(Error::InvalidRequest {
					message: format!("source_refs[{index}] must be a non-empty object."),
				});
			},
		}
	}

	let value = Value::Array(source_refs.to_vec());

	validation::validate_json_strings(&value, "$.source_refs")?;

	Ok(value)
}
