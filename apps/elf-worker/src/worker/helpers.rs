use crate::worker::{
	BASE_BACKOFF_MS, Chunk, ChunkRecord, Error, MAX_BACKOFF_MS, MAX_OUTBOX_ERROR_CHARS, MemoryNote,
	OffsetDateTime, ProjectDocRefFields, QdrantError, Result, Rfc3339, Serialize, ToString, Uuid,
	Value,
};

pub(super) fn is_not_found_error(err: &QdrantError) -> bool {
	let message = err.to_string().to_lowercase();
	let point_not_found =
		(message.contains("not found") || message.contains("404")) && message.contains("point");
	let no_point_found = message.contains("no point") && message.contains("found");

	point_not_found || no_point_found
}

pub(super) fn note_is_active(note: &MemoryNote, now: OffsetDateTime) -> bool {
	if !note.status.eq_ignore_ascii_case("active") {
		return false;
	}

	if let Some(expires_at) = note.expires_at
		&& expires_at <= now
	{
		return false;
	}

	true
}

pub(super) fn build_chunk_records(note_id: Uuid, chunks: &[Chunk]) -> Result<Vec<ChunkRecord>> {
	let mut records = Vec::with_capacity(chunks.len());

	for chunk in chunks {
		let start_offset = to_i32(chunk.start_offset, "start_offset")?;
		let end_offset = to_i32(chunk.end_offset, "end_offset")?;

		records.push(ChunkRecord {
			chunk_id: chunk_id_for(note_id, chunk.chunk_index),
			chunk_index: chunk.chunk_index,
			start_offset,
			end_offset,
			text: chunk.text.clone(),
		});
	}

	Ok(records)
}

pub(super) fn chunk_id_for(note_id: Uuid, chunk_index: i32) -> Uuid {
	let name = format!("{note_id}:{chunk_index}");

	Uuid::new_v5(&Uuid::NAMESPACE_OID, name.as_bytes())
}

pub(super) fn to_i32(value: usize, label: &str) -> Result<i32> {
	i32::try_from(value).map_err(|_| {
		Error::Validation(format!("Chunk {label} offset {value} exceeds supported range."))
	})
}

pub(super) fn mean_pool(chunks: &[Vec<f32>]) -> Option<Vec<f32>> {
	if chunks.is_empty() {
		return None;
	}

	let dim = chunks[0].len();
	let mut out = vec![0.0_f32; dim];

	for vec in chunks {
		for (idx, value) in vec.iter().enumerate() {
			out[idx] += value;
		}
	}
	for value in &mut out {
		*value /= chunks.len() as f32;
	}

	Some(out)
}

pub(super) fn format_timestamp(ts: OffsetDateTime) -> Result<String> {
	ts.format(&Rfc3339).map_err(|_| Error::Message("Failed to format timestamp.".to_string()))
}

pub(super) fn validate_vector_dim(vec: &[f32], expected_dim: u32) -> Result<()> {
	if vec.len() != expected_dim as usize {
		return Err(Error::Validation(format!(
			"Embedding dimension {} does not match configured vector_dim {}.",
			vec.len(),
			expected_dim
		)));
	}

	Ok(())
}

pub(super) fn format_vector_text(vec: &[f32]) -> String {
	let mut out = String::from("[");

	for (idx, value) in vec.iter().enumerate() {
		if idx > 0 {
			out.push(',');
		}

		out.push_str(&value.to_string());
	}

	out.push(']');

	out
}

pub(super) fn encode_json<T>(value: &T, label: &str) -> Result<Value>
where
	T: Serialize,
{
	serde_json::to_value(value)
		.map_err(|err| Error::Message(format!("Failed to encode {label}: {err}.")))
}

pub(super) fn sanitize_outbox_error(text: &str) -> String {
	let mut parts = Vec::new();
	let mut redact_next = false;

	for raw in text.split_whitespace() {
		let mut word = raw.to_string();

		if redact_next {
			word = "[REDACTED]".to_string();
			redact_next = false;
		}
		if raw.eq_ignore_ascii_case("bearer") {
			redact_next = true;
		}

		let lowered = raw.to_ascii_lowercase();

		for key in ["api_key", "apikey", "password", "secret", "token"] {
			if lowered.contains(key) && (lowered.contains('=') || lowered.contains(':')) {
				let sep = if raw.contains('=') { '=' } else { ':' };
				let prefix = match raw.split(sep).next() {
					Some(prefix) => prefix,
					None => raw,
				};

				word = format!("{prefix}{sep}[REDACTED]");

				break;
			}
		}

		parts.push(word);
	}

	let mut out = parts.join(" ");

	if out.chars().count() > MAX_OUTBOX_ERROR_CHARS {
		out = out.chars().take(MAX_OUTBOX_ERROR_CHARS).collect();

		out.push_str("...");
	}

	out
}

pub(super) fn backoff_for_attempt(attempt: i32) -> time::Duration {
	let attempts = attempt.max(1) as u32;
	let exp = attempts.saturating_sub(1).min(6);
	let base = BASE_BACKOFF_MS.saturating_mul(1 << exp);
	let capped = base.min(MAX_BACKOFF_MS);

	time::Duration::milliseconds(capped)
}

pub(super) fn to_std_duration(duration: time::Duration) -> std::time::Duration {
	let millis = duration.whole_milliseconds();

	if millis <= 0 {
		return std::time::Duration::from_millis(0);
	}

	std::time::Duration::from_millis(millis as u64)
}

pub(super) fn project_doc_ref_fields(
	source_ref: &Value,
	fallback_timestamp: OffsetDateTime,
	doc_type: &str,
) -> Result<ProjectDocRefFields> {
	let source_ref_field = |field_name: &str| -> Option<String> {
		source_ref
			.get(field_name)
			.and_then(Value::as_str)
			.filter(|value| !value.is_empty())
			.map(ToString::to_string)
	};
	let doc_ts = match source_ref
		.get("ts")
		.and_then(Value::as_str)
		.filter(|value| OffsetDateTime::parse(value, &Rfc3339).is_ok())
		.map(ToString::to_string)
		.or_else(|| {
			source_ref
				.get("doc_ts")
				.and_then(Value::as_str)
				.filter(|value| OffsetDateTime::parse(value, &Rfc3339).is_ok())
				.map(ToString::to_string)
		}) {
		Some(value) => value,
		None => format_timestamp(fallback_timestamp)?,
	};
	let thread_id = if doc_type == "chat" { source_ref_field("thread_id") } else { None };
	let domain = if doc_type == "search" { source_ref_field("domain") } else { None };
	let repo = if doc_type == "dev" { source_ref_field("repo") } else { None };

	Ok((doc_ts, thread_id, domain, repo))
}
