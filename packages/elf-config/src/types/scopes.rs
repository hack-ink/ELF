use serde::Deserialize;

/// Scope labels and access policy used by memory operations.
#[derive(Debug, Deserialize)]
pub struct Scopes {
	/// All scope labels allowed by this deployment.
	pub allowed: Vec<String>,
	/// Scope sets referenced by named read profiles.
	pub read_profiles: ReadProfiles,
	/// Relative precedence used when multiple scopes are eligible.
	pub precedence: ScopePrecedence,
	/// Scope-level write permissions.
	pub write_allowed: ScopeWriteAllowed,
}

/// Scope lists used by named read profiles.
#[derive(Debug, Deserialize)]
pub struct ReadProfiles {
	/// Scope set for `private_only`.
	pub private_only: Vec<String>,
	/// Scope set for `private_plus_project`.
	pub private_plus_project: Vec<String>,
	/// Scope set for `all_scopes`.
	pub all_scopes: Vec<String>,
}

/// Integer precedence used to break ties between scope classes.
#[derive(Debug, Deserialize)]
pub struct ScopePrecedence {
	/// Precedence assigned to `agent_private`.
	pub agent_private: i32,
	/// Precedence assigned to `project_shared`.
	pub project_shared: i32,
	/// Precedence assigned to `org_shared`.
	pub org_shared: i32,
}

/// Scope-level write toggles.
#[derive(Debug, Deserialize)]
pub struct ScopeWriteAllowed {
	/// Whether writes to `agent_private` are allowed.
	pub agent_private: bool,
	/// Whether writes to `project_shared` are allowed.
	pub project_shared: bool,
	/// Whether writes to `org_shared` are allowed.
	pub org_shared: bool,
}
