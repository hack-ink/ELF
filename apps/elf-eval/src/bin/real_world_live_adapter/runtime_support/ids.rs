use blake3::Hasher;

pub(crate) fn project_id_for_job(job_id: &str) -> String {
	format!("job-{}", slug(job_id))
}

pub(crate) fn slug(value: &str) -> String {
	let mut out = String::new();
	let mut last_dash = false;

	for ch in value.chars() {
		if ch.is_ascii_alphanumeric() {
			out.push(ch.to_ascii_lowercase());

			last_dash = false;
		} else if !last_dash && !out.is_empty() {
			out.push('-');

			last_dash = true;
		}
	}

	while out.ends_with('-') {
		out.pop();
	}

	if out.is_empty() { "item".to_string() } else { out }
}

pub(crate) fn short_hash(value: &str) -> String {
	let mut hasher = Hasher::new();

	hasher.update(value.as_bytes());

	hasher.finalize().to_hex().chars().take(12).collect()
}

pub(crate) fn push_unique(values: &mut Vec<String>, value: String) {
	if !values.iter().any(|existing| existing == &value) {
		values.push(value);
	}
}
