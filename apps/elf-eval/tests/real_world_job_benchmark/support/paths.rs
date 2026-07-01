use std::{
	env,
	path::{Path, PathBuf},
};

use color_eyre::{Result, eyre};

pub(crate) fn fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_memory")
		.join("work_resume")
}

pub(crate) fn fixture_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join("real_world_memory")
}

pub(crate) fn real_world_memory_fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join("real_world_memory")
}

pub(crate) fn evolution_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("evolution")
}

pub(crate) fn operator_debug_fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_job")
		.join("operator_debugging_ux")
}

pub(crate) fn project_decisions_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("project_decisions")
}

pub(crate) fn retrieval_fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_memory")
		.join("retrieval")
}

pub(crate) fn capture_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("capture_integration")
}

pub(crate) fn consolidation_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("consolidation")
}

pub(crate) fn memory_summary_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("memory_summary")
}

pub(crate) fn proactive_brief_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("proactive_brief")
}

pub(crate) fn scheduled_memory_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("scheduled_memory")
}

pub(crate) fn work_continuity_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("work_continuity")
}

pub(crate) fn knowledge_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("knowledge")
}

pub(crate) fn source_library_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("source_library")
}

pub(crate) fn production_ops_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("production_ops")
}

pub(crate) fn core_archival_memory_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("core_archival_memory")
}

pub(crate) fn context_trajectory_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("context_trajectory")
}

pub(crate) fn adversarial_quality_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("adversarial_quality")
}

pub(crate) fn graph_rag_external_fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_external_adapters")
		.join("graph_rag")
}

pub(crate) fn workspace_root() -> Result<PathBuf> {
	let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
	let root = manifest_dir
		.parent()
		.and_then(Path::parent)
		.ok_or_else(|| eyre::eyre!("could not resolve workspace root"))?;

	Ok(root.to_path_buf())
}

pub(crate) fn collapse_whitespace(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}
