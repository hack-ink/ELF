use std::fs;

use color_eyre::Result;

pub(crate) fn make_task_catalog() -> Result<String> {
	let workspace = super::workspace_root()?;
	let makefiles_dir = workspace.join("makefiles");
	let mut catalog = fs::read_to_string(workspace.join("Makefile.toml"))?;

	if makefiles_dir.is_dir() {
		let mut paths = Vec::new();

		for entry in fs::read_dir(makefiles_dir)? {
			paths.push(entry?.path());
		}

		paths.sort();

		for path in paths {
			if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
				catalog.push('\n');
				catalog.push_str(fs::read_to_string(path)?.as_str());
			}
		}
	}

	Ok(catalog)
}
