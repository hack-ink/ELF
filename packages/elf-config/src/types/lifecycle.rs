use serde::Deserialize;

/// Lifecycle retention and purge settings.
#[derive(Debug, Deserialize)]
pub struct Lifecycle {
	/// Note-type-specific TTL settings.
	pub ttl_days: TtlDays,
	/// Days to retain deleted notes before purge.
	pub purge_deleted_after_days: i64,
	/// Days to retain deprecated notes before purge.
	pub purge_deprecated_after_days: i64,
}

/// TTL values in days for each note type.
#[derive(Debug, Deserialize)]
pub struct TtlDays {
	/// TTL for `plan` notes.
	pub plan: i64,
	/// TTL for `fact` notes.
	pub fact: i64,
	/// TTL for `preference` notes.
	pub preference: i64,
	/// TTL for `constraint` notes.
	pub constraint: i64,
	/// TTL for `decision` notes.
	pub decision: i64,
	/// TTL for `profile` notes.
	pub profile: i64,
}
