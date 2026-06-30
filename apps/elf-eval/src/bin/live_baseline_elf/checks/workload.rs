use crate::{
	AGENT_ID, AddNoteInput, AddNoteRequest, PROJECT_ID, QueryCase, SCOPE, SoakConfig, TENANT_ID,
	checks, env,
};

pub(crate) fn concurrent_note_count() -> usize {
	if let Ok(value) = env::var("ELF_BASELINE_CONCURRENT_NOTES")
		&& let Ok(parsed) = value.parse::<usize>()
	{
		return parsed.max(1);
	}

	match env::var("ELF_BASELINE_PROFILE").as_deref() {
		Ok("backfill" | "large") => 32,
		Ok("stress") => 32,
		Ok("scale" | "full") => 16,
		_ => 4,
	}
}

pub(crate) fn concurrent_add_request(index: usize) -> AddNoteRequest {
	let marker = concurrent_marker(index);

	AddNoteRequest {
		tenant_id: TENANT_ID.to_string(),
		project_id: PROJECT_ID.to_string(),
		agent_id: AGENT_ID.to_string(),
		scope: SCOPE.to_string(),
		notes: vec![AddNoteInput {
			r#type: "fact".to_string(),
			key: Some(format!("concurrent_{index:03}")),
			text: format!(
				"Concurrent benchmark note {index:03} records marker `{marker}` for write race validation."
			),
			structured: None,
			importance: 0.91,
			confidence: 0.96,
			ttl_days: None,
			source_ref: serde_json::json!({
				"source": "ELF live baseline concurrent write check",
				"document": format!("concurrent-{index:03}.md"),
			}),
			write_policy: None,
		}],
	}
}

pub(crate) fn concurrent_query_case(index: usize) -> QueryCase {
	let marker = concurrent_marker(index);

	QueryCase::generated(
		format!("concurrent-{index:03}"),
		format!("Find the concurrent benchmark note containing marker {marker}."),
		format!("concurrent-{index:03}.md"),
		vec![marker],
	)
}

pub(crate) fn soak_config() -> SoakConfig {
	let profile = env::var("ELF_BASELINE_PROFILE").ok();
	let (default_seconds, default_rounds) = match profile.as_deref() {
		Some("backfill" | "large") => (60, 6),
		Some("stress") => (60, 6),
		Some("scale" | "full") => (15, 3),
		_ => (0, 0),
	};

	SoakConfig {
		target_seconds: checks::parse_env_u64("ELF_BASELINE_SOAK_SECONDS")
			.unwrap_or(default_seconds),
		write_rounds: checks::parse_env_usize("ELF_BASELINE_SOAK_ROUNDS").unwrap_or(default_rounds),
		probe_interval_millis: checks::parse_env_u64("ELF_BASELINE_SOAK_PROBE_INTERVAL_MS")
			.unwrap_or(1_000)
			.max(100),
	}
}

pub(crate) fn soak_add_request(index: usize) -> AddNoteRequest {
	let marker = soak_marker(index);
	let (topic, detail) = soak_topic(index);

	AddNoteRequest {
		tenant_id: TENANT_ID.to_string(),
		project_id: PROJECT_ID.to_string(),
		agent_id: AGENT_ID.to_string(),
		scope: SCOPE.to_string(),
		notes: vec![AddNoteInput {
			r#type: "fact".to_string(),
			key: Some(format!("soak_{index:03}")),
			text: format!(
				"Soak benchmark note {index:03} covers {topic}. {detail} It records stability marker `{marker}` for repeated worker and search probes."
			),
			structured: None,
			importance: 0.92,
			confidence: 0.97,
			ttl_days: None,
			source_ref: serde_json::json!({
				"source": "ELF live baseline soak stability check",
				"document": format!("soak-{index:03}.md"),
			}),
			write_policy: None,
		}],
	}
}

pub(crate) fn soak_query_case(index: usize) -> QueryCase {
	let marker = soak_marker(index);
	let (topic, _) = soak_topic(index);

	QueryCase::generated(
		format!("soak-{index:03}"),
		format!("Find the soak benchmark note about {topic} containing marker {marker}."),
		format!("soak-{index:03}.md"),
		vec![marker],
	)
}

pub(crate) fn concurrency_probe_indexes(note_count: usize) -> Vec<usize> {
	let mut indexes = vec![0, note_count / 2, note_count.saturating_sub(1)];

	indexes.sort_unstable();
	indexes.dedup();

	indexes
}

fn concurrent_marker(index: usize) -> String {
	format!("concurrency-{}-{index:03}", marker_word(index))
}

fn soak_marker(index: usize) -> String {
	format!("soak-stability-{}-{index:03}", marker_word(index))
}

fn marker_word(index: usize) -> &'static str {
	const WORDS: &[&str] = &[
		"aurora", "banyan", "cobalt", "delta", "ember", "fennel", "granite", "harbor", "indigo",
		"jasper", "keystone", "lantern", "meridian", "nebula", "onyx", "prairie", "quartz",
		"raven", "solstice", "topaz", "umbra", "verdant", "willow", "xenon", "yarrow", "zephyr",
		"atlas", "beacon", "citadel", "drift", "equinox", "forge",
	];

	WORDS[index % WORDS.len()]
}

fn soak_topic(index: usize) -> (&'static str, &'static str) {
	const TOPICS: &[(&str, &str)] = &[
		(
			"release rollback fencing",
			"The rollback controller waits for a signed deploy fence before the next canary.",
		),
		(
			"invoice export batching",
			"The exporter groups invoice CSV rows by merchant ledger before upload.",
		),
		("search shard warming", "The search router warms tenant shard caches before rank probes."),
		(
			"incident pager routing",
			"The incident desk routes page ownership through the release captain.",
		),
		(
			"backup restore rehearsal",
			"The restore rehearsal checks WAL freshness before dry-run recovery.",
		),
		(
			"feature flag expiry",
			"The flag sweeper archives expired toggles before deleting rollout rules.",
		),
		(
			"support queue triage",
			"The support classifier separates billing tickets from access tickets.",
		),
		(
			"analytics job watermark",
			"The analytics worker stores a warehouse watermark after each import.",
		),
	];

	TOPICS[index % TOPICS.len()]
}
