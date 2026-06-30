mod competitor_retest;
mod letta_core;
mod openviking_trajectory;
mod qmd_debug_retest;
mod review_queue;
mod service_native;
mod temporal_reconciliation;

use std::{fs, path::Path};

use color_eyre::Result;

pub(crate) fn read_rust_module_sources(src_dir: &Path, module_name: &str) -> Result<String> {
	let module_root = src_dir.join(format!("{module_name}.rs"));
	let module_dir = src_dir.join(module_name);
	let mut source = fs::read_to_string(module_root)?;

	if module_dir.is_dir() {
		append_rust_sources(module_dir.as_path(), &mut source)?;
	}

	Ok(source)
}

fn append_rust_sources(dir: &Path, source: &mut String) -> Result<()> {
	let mut entries = Vec::new();

	for entry in fs::read_dir(dir)? {
		entries.push(entry?.path());
	}

	entries.sort();

	for path in entries {
		if path.is_dir() {
			append_rust_sources(path.as_path(), source)?;
		} else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
			source.push('\n');
			source.push_str(fs::read_to_string(path)?.as_str());
		}
	}

	Ok(())
}
