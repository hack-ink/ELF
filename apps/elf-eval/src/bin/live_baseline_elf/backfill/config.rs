use crate::env;

pub(crate) fn worker_concurrency() -> usize {
	let default = match env::var("ELF_BASELINE_PROFILE").as_deref() {
		Ok("backfill" | "large") => 4,
		Ok("stress") => 4,
		Ok("scale" | "full") => 2,
		_ => 1,
	};

	crate::parse_env_usize("ELF_BASELINE_WORKER_CONCURRENCY").unwrap_or(default).clamp(1, 32)
}

pub(super) fn backfill_batch_size() -> usize {
	crate::parse_env_usize("ELF_BASELINE_BACKFILL_BATCH_SIZE").unwrap_or(32).max(1)
}

pub(super) fn backfill_resume_probe_enabled() -> bool {
	env::var("ELF_BASELINE_BACKFILL_RESUME_PROBE")
		.map(|value| value != "0" && !value.eq_ignore_ascii_case("false"))
		.unwrap_or(true)
}

pub(super) fn backfill_interrupt_after(source_count: usize) -> Option<usize> {
	if !backfill_resume_probe_enabled() || source_count <= 1 {
		return None;
	}

	let configured = crate::parse_env_usize("ELF_BASELINE_BACKFILL_INTERRUPT_AFTER");
	let default = (source_count / 2).max(1);

	Some(configured.unwrap_or(default).clamp(1, source_count.saturating_sub(1)))
}
