mod declared;
mod job;
mod support;

use crate::{LoadedJob, MaterializationStatus, MaterializedJob, MaterializedJobInput};

pub(super) fn materialized_job(
	loaded: &LoadedJob,
	adapter_id: &str,
	input: MaterializedJobInput,
) -> MaterializedJob {
	job::materialized_job(loaded, adapter_id, input)
}

pub(super) fn declared_encoding_job(
	adapter_id: &str,
	loaded: &LoadedJob,
) -> Option<MaterializedJob> {
	declared::declared_encoding_job(adapter_id, loaded)
}

pub(super) fn not_encoded_job(adapter_id: &str, loaded: &LoadedJob) -> Option<MaterializedJob> {
	declared::not_encoded_job(adapter_id, loaded)
}

pub(super) fn is_elf_dreaming_readback_live_adapter(adapter_id: &str, suite: &str) -> bool {
	support::is_elf_dreaming_readback_live_adapter(adapter_id, suite)
}

pub(super) fn materialized_declared_status_job(
	adapter_id: &str,
	loaded: &LoadedJob,
	status: MaterializationStatus,
	reason: String,
) -> MaterializedJob {
	declared::materialized_declared_status_job(adapter_id, loaded, status, reason)
}
