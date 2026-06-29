use std::{fs, path::Path};

use color_eyre::{Result, eyre};
use serde::Serialize;

use super::types::RadarCursor;

pub(super) fn read_cursor(path: &Path) -> Result<RadarCursor> {
	let raw = fs::read_to_string(path)
		.map_err(|err| eyre::eyre!("failed to read cursor {}: {err}", path.display()))?;
	let cursor = serde_json::from_str(&raw)
		.map_err(|err| eyre::eyre!("failed to parse cursor {}: {err}", path.display()))?;

	Ok(cursor)
}

pub(super) fn write_json<T>(path: &Path, value: &T) -> Result<()>
where
	T: Serialize,
{
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)?;
	}

	let raw = serde_json::to_string_pretty(value)?;

	fs::write(path, format!("{raw}\n"))?;

	Ok(())
}

pub(super) fn write_text(path: &Path, content: &str) -> Result<()> {
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)?;
	}

	fs::write(path, content)?;

	Ok(())
}
