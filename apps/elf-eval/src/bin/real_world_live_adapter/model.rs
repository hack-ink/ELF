use super::*;

#[path = "model/cli.rs"] mod cli;
#[path = "model/consolidation.rs"] mod consolidation;
#[path = "model/live.rs"] mod live;
#[path = "model/materialization.rs"] mod materialization;
#[path = "model/providers.rs"] mod providers;
#[path = "model/runtime.rs"] mod runtime;

pub(super) use cli::*;
pub(super) use consolidation::*;
pub(super) use live::*;
pub(super) use materialization::*;
pub(super) use providers::*;
pub(super) use runtime::*;

pub(super) const JOB_SCHEMA: &str = "elf.real_world_job/v1";
pub(super) const EVIDENCE_SCHEMA: &str = "elf.real_world_live_adapter_materialization/v1";
pub(super) const TENANT_ID: &str = "elf-live-real-world";
pub(super) const AGENT_ID: &str = "elf-live-real-world-agent";
pub(super) const SCOPE: &str = "agent_private";
pub(super) const ELF_NOTE_CHUNK_CHARS: usize = 220;
