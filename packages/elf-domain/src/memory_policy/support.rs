mod lifecycle;
mod memory;
mod providers;
mod ranking;
mod scopes;
mod search;
mod service_storage;

use elf_config::{Config, MemoryPolicy};

pub(crate) fn test_config(policy: MemoryPolicy) -> Config {
	let mut cfg = service_storage::test_default_config();

	cfg.memory.policy = policy;

	cfg
}
