use elf_config::{ReadProfiles, ScopePrecedence, ScopeWriteAllowed, Scopes};

pub(crate) fn memory_policy_scopes_config() -> Scopes {
	Scopes {
		allowed: vec!["agent_private".to_string()],
		read_profiles: ReadProfiles {
			private_only: vec!["agent_private".to_string()],
			private_plus_project: vec!["agent_private".to_string()],
			all_scopes: vec!["agent_private".to_string()],
		},
		precedence: ScopePrecedence { agent_private: 30, project_shared: 20, org_shared: 10 },
		write_allowed: ScopeWriteAllowed {
			agent_private: true,
			project_shared: true,
			org_shared: true,
		},
	}
}
