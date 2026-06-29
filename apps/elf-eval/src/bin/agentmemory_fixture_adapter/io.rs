use std::{fs, path::PathBuf};

use color_eyre::Result;

use crate::types::AgentmemoryFixture;

pub(super) fn read_fixture(path: &PathBuf) -> Result<AgentmemoryFixture> {
	let raw = fs::read_to_string(path)?;
	let fixture = serde_json::from_str(&raw)?;

	Ok(fixture)
}

pub(super) fn write_output(path: PathBuf, json: &str) -> Result<()> {
	if let Some(parent) = path.parent()
		&& !parent.as_os_str().is_empty()
	{
		fs::create_dir_all(parent)?;
	}

	fs::write(path, json)?;

	Ok(())
}
