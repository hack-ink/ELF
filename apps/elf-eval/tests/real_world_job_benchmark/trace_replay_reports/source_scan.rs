use std::{fs, path::Path};

use color_eyre::Result;

pub(crate) fn graph_report_service_sources(workspace: &Path) -> Result<String> {
	let mut source =
		fs::read_to_string(workspace.join("packages/elf-service/src/graph_report.rs"))?;

	append_rust_sources(
		workspace.join("packages/elf-service/src/graph_report").as_path(),
		&mut source,
	)?;

	Ok(source)
}

pub(crate) fn mcp_server_sources(workspace: &Path) -> Result<String> {
	let mut source = fs::read_to_string(workspace.join("apps/elf-mcp/src/app/server.rs"))?;

	append_rust_sources(workspace.join("apps/elf-mcp/src/app/server").as_path(), &mut source)?;

	Ok(source)
}

pub(crate) fn api_route_sources(workspace: &Path) -> Result<String> {
	let mut source = fs::read_to_string(workspace.join("apps/elf-api/src/routes.rs"))?;

	append_rust_sources(workspace.join("apps/elf-api/src/routes").as_path(), &mut source)?;

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
