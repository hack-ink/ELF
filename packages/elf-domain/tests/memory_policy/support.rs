mod support_lifecycle;
mod support_memory;
mod support_providers;
mod support_ranking;
mod support_scopes;
mod support_search;
mod support_service_storage;

use elf_config::{Config, MemoryPolicy};

pub(crate) fn memory_policy_config(policy: MemoryPolicy) -> Config {
	let mut cfg = support_service_storage::memory_policy_default_config();

	cfg.memory.policy = policy;

	cfg
}
