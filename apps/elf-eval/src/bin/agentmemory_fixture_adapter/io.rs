use std::{fs, path::PathBuf};

use super::types::AgentmemoryFixture;

pub(super) fn read_fixture(path: &PathBuf) -> color_eyre::Result<AgentmemoryFixture> {
	let raw = fs::read_to_string(path)?;
	let fixture = serde_json::from_str(&raw)?;

	Ok(fixture)
}

pub(super) fn write_output(path: PathBuf, json: &str) -> color_eyre::Result<()> {
	if let Some(parent) = path.parent()
		&& !parent.as_os_str().is_empty()
	{
		fs::create_dir_all(parent)?;
	}

	fs::write(path, json)?;

	Ok(())
}
