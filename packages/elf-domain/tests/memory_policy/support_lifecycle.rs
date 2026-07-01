use elf_config::{Lifecycle, Security, TtlDays};

pub(crate) fn memory_policy_lifecycle_config() -> Lifecycle {
	Lifecycle {
		ttl_days: TtlDays {
			plan: 14,
			fact: 180,
			preference: 0,
			constraint: 0,
			decision: 0,
			profile: 0,
		},
		purge_deleted_after_days: 30,
		purge_deprecated_after_days: 180,
	}
}

pub(crate) fn memory_policy_security_config() -> Security {
	Security {
		bind_localhost_only: true,
		reject_non_english: true,
		redact_secrets_on_write: true,
		evidence_min_quotes: 1,
		evidence_max_quotes: 2,
		evidence_max_quote_chars: 320,
		auth_mode: "off".to_string(),
		auth_keys: vec![],
	}
}
