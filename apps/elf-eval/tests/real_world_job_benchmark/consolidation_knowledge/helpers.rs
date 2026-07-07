use std::{fs, path::Path};

use color_eyre::Result;
pub(super) fn real_world_live_adapter_sources(workspace: &Path) -> Result<String> {
	let mut source = fs::read_to_string(
		workspace.join("apps/elf-eval/src/bin/real_world_live_adapter/main.rs"),
	)?;

	append_rust_sources(
		workspace.join("apps/elf-eval/src/bin/real_world_live_adapter").as_path(),
		&mut source,
	)?;

	Ok(source)
}

pub(super) fn real_world_job_benchmark_sources(workspace: &Path) -> Result<String> {
	let mut source = fs::read_to_string(
		workspace.join("apps/elf-eval/src/bin/real_world_job_benchmark/main.rs"),
	)?;

	append_rust_sources(
		workspace.join("apps/elf-eval/src/bin/real_world_job_benchmark").as_path(),
		&mut source,
	)?;

	Ok(source)
}

pub(super) fn append_rust_sources(dir: &Path, source: &mut String) -> Result<()> {
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
