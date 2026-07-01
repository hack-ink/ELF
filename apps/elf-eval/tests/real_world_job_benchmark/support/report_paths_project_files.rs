use std::path::PathBuf;

use color_eyre::Result;

use crate::support;

pub(crate) fn readme_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?.join("README.md"))
}
