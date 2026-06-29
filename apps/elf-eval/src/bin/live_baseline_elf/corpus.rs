use crate::{Command, CorpusNote, HashSet, Path, PathBuf, QueryCase, QueryManifest, env, eyre, fs};

pub(super) fn load_corpus_notes(corpus_dir: &Path) -> color_eyre::Result<Vec<CorpusNote>> {
	let mut paths = fs::read_dir(corpus_dir)?
		.map(|entry| entry.map(|entry| entry.path()))
		.collect::<std::io::Result<Vec<_>>>()?;

	paths.retain(|path| {
		path.extension()
			.and_then(|ext| ext.to_str())
			.is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
	});
	paths.sort();

	let mut out = Vec::with_capacity(paths.len());

	for path in paths {
		let source_doc = path
			.file_name()
			.and_then(|name| name.to_str())
			.ok_or_else(|| {
				eyre::eyre!("Corpus path has no valid UTF-8 file name: {}", path.display())
			})?
			.to_string();
		let raw = fs::read_to_string(&path)?;
		let title = title_from_markdown(&raw, &source_doc);
		let text = raw
			.lines()
			.filter(|line| !line.trim_start().starts_with('#'))
			.collect::<Vec<_>>()
			.join(" ")
			.split_whitespace()
			.collect::<Vec<_>>()
			.join(" ");

		out.push(CorpusNote { key: key_for_doc(&source_doc), title, text, source_doc });
	}

	if out.is_empty() {
		return Err(eyre::eyre!("No markdown corpus files found in {}.", corpus_dir.display()));
	}

	Ok(out)
}

pub(super) fn load_queries(path: &PathBuf) -> color_eyre::Result<QueryManifest> {
	let raw = fs::read_to_string(path)?;

	Ok(serde_json::from_str(&raw)?)
}

pub(super) fn title_from_markdown(raw: &str, source_doc: &str) -> String {
	raw.lines()
		.find_map(|line| line.trim_start().strip_prefix("# "))
		.map(str::trim)
		.filter(|title| !title.is_empty())
		.map(str::to_string)
		.unwrap_or_else(|| source_doc.to_string())
}

pub(super) fn key_for_doc(doc: &str) -> String {
	let stem = Path::new(doc).file_stem().and_then(|stem| stem.to_str()).unwrap_or(doc);
	let mut key = String::with_capacity(stem.len());
	let mut last_was_separator = false;

	for ch in stem.chars() {
		if ch.is_ascii_alphanumeric() {
			key.push(ch.to_ascii_lowercase());

			last_was_separator = false;
		} else if !last_was_separator && !key.is_empty() {
			key.push('_');

			last_was_separator = true;
		}
	}

	if key.ends_with('_') {
		key.pop();
	}

	if key.is_empty() { "doc".to_string() } else { key }
}

pub(super) fn evidence_id_for_doc(doc: &str) -> String {
	Path::new(doc).file_stem().and_then(|stem| stem.to_str()).unwrap_or(doc).to_string()
}

pub(super) fn expected_docs_for_case(case: &QueryCase) -> Vec<String> {
	let mut docs = Vec::with_capacity(case.allowed_alternate_docs.len().saturating_add(1));

	docs.push(case.expected_doc.clone());
	docs.extend(case.allowed_alternate_docs.iter().cloned());

	docs
}

pub(super) fn embed_text(text: &str, vector_dim: u32) -> Vec<f32> {
	let dim = vector_dim as usize;
	let mut vector = vec![0.0_f32; dim];

	if dim == 0 {
		return vector;
	}

	let normalized = normalize_ascii_alnum_lowercase(text);

	for term in normalized.split_whitespace() {
		if term.len() < 2 {
			continue;
		}

		let hash = blake3::hash(term.as_bytes());
		let bytes = hash.as_bytes();
		let idx = (u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize) % dim;
		let sign = if bytes[4] & 1 == 0 { 1.0 } else { -1.0 };

		vector[idx] += sign;
	}

	if vector.iter().all(|value| *value == 0.0) {
		let hash = blake3::hash(text.as_bytes());
		let bytes = hash.as_bytes();
		let idx = (u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize) % dim;

		vector[idx] = 1.0;
	}

	let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();

	if norm > 0.0 {
		for value in &mut vector {
			*value /= norm;
		}
	}

	vector
}

pub(super) fn normalize_ascii_alnum_lowercase(text: &str) -> String {
	let mut normalized = String::with_capacity(text.len());

	for ch in text.chars() {
		if ch.is_ascii_alphanumeric() {
			normalized.push(ch.to_ascii_lowercase());
		} else {
			normalized.push(' ');
		}
	}

	normalized
}

pub(super) fn terms(text: &str) -> HashSet<String> {
	text.split(|ch: char| !ch.is_ascii_alphanumeric())
		.map(str::trim)
		.filter(|term| !term.is_empty())
		.map(str::to_ascii_lowercase)
		.collect()
}

pub(super) fn distinctive_terms(text: &str, limit: usize) -> Vec<String> {
	let stop_words = [
		"the", "and", "for", "with", "that", "this", "from", "into", "must", "uses", "after",
		"before", "query", "memory", "note",
	];
	let stop_words = stop_words.into_iter().collect::<HashSet<_>>();
	let mut out = Vec::new();

	for raw in text.split(|ch: char| !ch.is_ascii_alphanumeric()) {
		let term = raw.trim();

		if term.len() < 5 {
			continue;
		}

		let lowered = term.to_ascii_lowercase();

		if stop_words.contains(lowered.as_str()) || out.iter().any(|existing| existing == term) {
			continue;
		}

		out.push(term.to_string());

		if out.len() >= limit {
			break;
		}
	}

	out
}

pub(super) fn contains_case_insensitive(haystack: &str, needle: &str) -> bool {
	haystack.to_ascii_lowercase().contains(&needle.to_ascii_lowercase())
}

pub(super) fn git_head() -> color_eyre::Result<String> {
	if let Ok(head) = env::var("ELF_BASELINE_ELF_HEAD") {
		let head = head.trim();

		if !head.is_empty() {
			return Ok(head.to_string());
		}
	}

	let output = Command::new("git").args(["rev-parse", "HEAD"]).output()?;

	if !output.status.success() {
		return Err(eyre::eyre!("git rev-parse HEAD failed."));
	}

	Ok(String::from_utf8(output.stdout)?.trim().to_string())
}
