use std::{fs, path::Path};

use color_eyre::{Result, eyre};
use serde_json::Value;

pub(crate) fn load_json(path: &Path) -> Result<Value> {
	Ok(serde_json::from_str::<Value>(&fs::read_to_string(path)?)?)
}

pub(crate) fn array_at<'a>(value: &'a Value, pointer: &str) -> Result<&'a Vec<Value>> {
	value
		.pointer(pointer)
		.and_then(Value::as_array)
		.ok_or_else(|| eyre::eyre!("missing array at {pointer}"))
}

pub(crate) fn find_by_field<'a>(
	items: &'a [Value],
	field: &str,
	expected: &str,
) -> Result<&'a Value> {
	items
		.iter()
		.find(|item| item.pointer(field).and_then(Value::as_str) == Some(expected))
		.ok_or_else(|| eyre::eyre!("missing item with {field} = {expected}"))
}

pub(crate) fn array_contains_str(value: &Value, pointer: &str, expected: &str) -> Result<bool> {
	Ok(array_at(value, pointer)?.iter().any(|item| item.as_str() == Some(expected)))
}

pub(crate) fn string_array_at(value: &Value, pointer: &str) -> Result<Vec<String>> {
	array_at(value, pointer)?
		.iter()
		.map(|item| {
			item.as_str()
				.map(str::to_owned)
				.ok_or_else(|| eyre::eyre!("non-string entry at {pointer}"))
		})
		.collect()
}

pub(crate) fn set_json_pointer(value: &mut Value, pointer: &str, replacement: Value) -> Result<()> {
	let target =
		value.pointer_mut(pointer).ok_or_else(|| eyre::eyre!("missing JSON pointer {pointer}"))?;

	*target = replacement;

	Ok(())
}
