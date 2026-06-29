use std::{
	collections::BTreeSet,
	fs::{self, OpenOptions},
	io::Write as _,
	path::Path,
	process::{Command, Stdio},
	sync::Arc,
};

use blake3::Hasher;
use color_eyre::{Result, eyre};

use crate::{
	BaselineRuntime, DeterministicEmbedding, ELF_NOTE_CHUNK_CHARS, NoopExtractor, QmdArgs,
	TokenOverlapRerank,
};
use elf_config::Config;
use elf_service::Providers;

pub(super) fn runtime_config(runtime: &BaselineRuntime) -> Result<Config> {
	let mut cfg = elf_config::load(&runtime.config_path)?;

	cfg.storage.postgres.dsn = runtime.dsn.clone();
	cfg.storage.postgres.pool_max_conns = 12;
	cfg.storage.qdrant.url = runtime.qdrant_url.clone();
	cfg.storage.qdrant.collection = runtime.collection.clone();
	cfg.storage.qdrant.docs_collection = runtime.docs_collection.clone();
	cfg.providers.embedding.provider_id = "local".to_string();
	cfg.providers.embedding.model = "local-hash".to_string();
	cfg.providers.embedding.dimensions = cfg.storage.qdrant.vector_dim;
	cfg.providers.rerank.provider_id = "local".to_string();
	cfg.providers.rerank.model = "local-token-overlap".to_string();
	cfg.providers.llm_extractor.provider_id = "disabled".to_string();
	cfg.providers.llm_extractor.model = "disabled".to_string();
	cfg.context = None;

	Ok(cfg)
}

pub(super) fn deterministic_providers(vector_dim: u32) -> Providers {
	Providers::new(
		Arc::new(DeterministicEmbedding { vector_dim }),
		Arc::new(TokenOverlapRerank),
		Arc::new(NoopExtractor),
	)
}

pub(super) fn run_qmd_command(
	label: &str,
	args: &QmdArgs,
	home_dir: &Path,
	qmd_args: &[&str],
	log_path: &Path,
) -> Result<String> {
	let mut command = Command::new("npx");

	command
		.current_dir(&args.qmd_dir)
		.env("HOME", home_dir)
		.env("XDG_CACHE_HOME", "/root/.cache")
		.env("QMD_FORCE_CPU", "1")
		.arg("tsx")
		.arg("src/cli/qmd.ts");

	for arg in qmd_args {
		command.arg(arg);
	}

	run_logged_command(label, &mut command, log_path)
}

pub(super) fn run_logged_shell(
	label: &str,
	cwd: &Path,
	script: &str,
	log_path: &Path,
) -> Result<()> {
	let mut command = Command::new("bash");

	command.current_dir(cwd).arg("-lc").arg(script);

	run_logged_command(label, &mut command, log_path).map(|_| ())
}

pub(super) fn run_logged_command(
	label: &str,
	command: &mut Command,
	log_path: &Path,
) -> Result<String> {
	if let Some(parent) = log_path.parent() {
		fs::create_dir_all(parent)?;
	}

	let command_debug = format!("{command:?}");
	let output = command.stdout(Stdio::piped()).stderr(Stdio::piped()).output()?;
	let stdout = String::from_utf8_lossy(&output.stdout).to_string();
	let stderr = String::from_utf8_lossy(&output.stderr).to_string();
	let mut log = OpenOptions::new().create(true).append(true).open(log_path)?;

	writeln!(log, "## {label}")?;
	writeln!(log, "$ {command_debug}")?;

	if !stdout.trim().is_empty() {
		writeln!(log, "\nstdout:\n{stdout}")?;
	}
	if !stderr.trim().is_empty() {
		writeln!(log, "\nstderr:\n{stderr}")?;
	}
	if !output.status.success() {
		return Err(eyre::eyre!(
			"{label} failed with status {}. Inspect {}.",
			output.status,
			log_path.display()
		));
	}

	Ok(stdout)
}

pub(super) fn project_id_for_job(job_id: &str) -> String {
	format!("job-{}", slug(job_id))
}

pub(super) fn slug(value: &str) -> String {
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

pub(super) fn short_hash(value: &str) -> String {
	let mut hasher = Hasher::new();

	hasher.update(value.as_bytes());

	hasher.finalize().to_hex().chars().take(12).collect()
}

pub(super) fn push_unique(values: &mut Vec<String>, value: String) {
	if !values.iter().any(|existing| existing == &value) {
		values.push(value);
	}
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

		vector[idx] += 1.0;
	}

	let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();

	if norm > 0.0 {
		for value in &mut vector {
			*value /= norm;
		}
	}

	vector
}

pub(super) fn terms(text: &str) -> BTreeSet<String> {
	normalize_ascii_alnum_lowercase(text)
		.split_whitespace()
		.filter(|term| term.len() >= 2)
		.map(ToString::to_string)
		.collect()
}

pub(super) fn normalize_ascii_alnum_lowercase(text: &str) -> String {
	text.chars()
		.map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { ' ' })
		.collect()
}

pub(super) fn note_text_chunks(text: &str) -> Vec<String> {
	let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");

	if normalized.chars().count() <= ELF_NOTE_CHUNK_CHARS {
		return vec![normalized];
	}

	let mut chunks = Vec::new();
	let mut current = String::new();

	for word in normalized.split_whitespace() {
		if word.chars().count() > ELF_NOTE_CHUNK_CHARS {
			if !current.is_empty() {
				chunks.push(current);

				current = String::new();
			}

			chunks.extend(split_long_token(word));

			continue;
		}

		let separator = usize::from(!current.is_empty());

		if current.chars().count() + separator + word.chars().count() > ELF_NOTE_CHUNK_CHARS
			&& !current.is_empty()
		{
			chunks.push(current);

			current = String::new();
		}
		if !current.is_empty() {
			current.push(' ');
		}

		current.push_str(word);
	}

	if !current.is_empty() {
		chunks.push(current);
	}

	chunks
}

fn split_long_token(token: &str) -> Vec<String> {
	let mut chunks = Vec::new();
	let mut current = String::new();

	for ch in token.chars() {
		if current.chars().count() >= ELF_NOTE_CHUNK_CHARS {
			chunks.push(current);

			current = String::new();
		}

		current.push(ch);
	}

	if !current.is_empty() {
		chunks.push(current);
	}

	chunks
}
