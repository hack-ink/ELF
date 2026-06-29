use serde::Deserialize;

/// Request security, evidence, and auth settings.
#[derive(Debug, Deserialize)]
pub struct Security {
	/// Whether services must bind only to loopback interfaces.
	pub bind_localhost_only: bool,
	/// Whether non-English input is rejected at the API boundary.
	pub reject_non_english: bool,
	/// Whether secret-like text is redacted before write.
	pub redact_secrets_on_write: bool,
	/// Minimum number of quotes required for evidence binding.
	pub evidence_min_quotes: u32,
	/// Maximum number of quotes allowed for evidence binding.
	pub evidence_max_quotes: u32,
	/// Maximum characters allowed in one evidence quote.
	pub evidence_max_quote_chars: u32,
	/// Authentication mode such as `off` or `static_keys`.
	pub auth_mode: String,
	/// Static bearer-token entries used when `auth_mode` is `static_keys`.
	pub auth_keys: Vec<SecurityAuthKey>,
}

/// A single static bearer-token entry.
#[derive(Debug, Deserialize)]
pub struct SecurityAuthKey {
	/// Stable token identifier used for auditing.
	pub token_id: String,
	/// Bearer token value matched from incoming requests.
	pub token: String,
	/// Tenant identifier granted by this token.
	pub tenant_id: String,
	/// Project identifier granted by this token.
	pub project_id: String,

	/// Optional agent identifier restriction.
	pub agent_id: Option<String>,
	/// Read profile granted by this token.
	pub read_profile: String,
	/// Role assigned to this token.
	pub role: SecurityAuthRole,
}

/// Role values accepted by static auth keys.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityAuthRole {
	/// Standard user token.
	User,
	/// Admin token with elevated write privileges.
	Admin,
	/// Super-admin token for global admin operations.
	SuperAdmin,
}
