use super::*;

fn current_rss_kb() -> Option<u64> {
	let status = fs::read_to_string("/proc/self/status").ok()?;

	status.lines().find_map(|line| {
		let rest = line.strip_prefix("VmHWM:")?.trim();
		let value = rest.split_whitespace().next()?;

		value.parse::<u64>().ok()
	})
}

fn path_size_bytes(path: &Path) -> color_eyre::Result<u64> {
	let metadata = fs::metadata(path)?;

	if metadata.is_file() {
		return Ok(metadata.len());
	}
	if !metadata.is_dir() {
		return Ok(0);
	}

	let mut bytes = 0_u64;

	for entry in fs::read_dir(path)? {
		let entry = entry?;

		bytes = bytes.saturating_add(path_size_bytes(&entry.path())?);
	}

	Ok(bytes)
}

pub(super) async fn resource_envelope_check_impl(
	service: &ElfService,
	corpus_dir: &Path,
	report_path: &Path,
	checkpoint_path: &Path,
	elapsed_seconds: f64,
) -> CheckResult {
	let max_elapsed_seconds = env::var("ELF_BASELINE_MAX_ELF_SECONDS")
		.ok()
		.and_then(|value| value.parse::<f64>().ok())
		.unwrap_or(600.0);
	let max_rss_kb = env::var("ELF_BASELINE_MAX_ELF_RSS_KB")
		.ok()
		.and_then(|value| value.parse::<u64>().ok())
		.unwrap_or(1_500_000);
	let rss_kb = current_rss_kb();
	let pass = elapsed_seconds <= max_elapsed_seconds && rss_kb.is_none_or(|rss| rss <= max_rss_kb);
	let postgres_database_bytes = postgres_database_bytes(service).await.ok();
	let corpus_dir_bytes = path_size_bytes(corpus_dir).unwrap_or_default();
	let report_dir_bytes = report_path.parent().and_then(|path| path_size_bytes(path).ok());
	let checkpoint_file_bytes = checkpoint_path.metadata().ok().map(|metadata| metadata.len());

	CheckResult {
		name: "resource_envelope",
		status: if pass { "pass" } else { "lifecycle_fail" },
		reason: if pass {
			"ELF live-baseline runtime stayed within the configured local resource envelope."
				.to_string()
		} else {
			"ELF live-baseline runtime exceeded the configured local resource envelope.".to_string()
		},
		evidence: serde_json::json!(ResourceEnvelopeEvidence {
			elapsed_seconds,
			max_elapsed_seconds,
			rss_kb,
			max_rss_kb,
			postgres_database_bytes,
			corpus_dir_bytes,
			report_dir_bytes,
			checkpoint_file_bytes,
		}),
	}
}

async fn postgres_database_bytes(service: &ElfService) -> color_eyre::Result<i64> {
	let bytes = sqlx::query_scalar::<_, i64>("SELECT pg_database_size(current_database())::bigint")
		.fetch_one(&service.db.pool)
		.await?;

	Ok(bytes)
}
