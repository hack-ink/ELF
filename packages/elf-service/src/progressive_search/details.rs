use std::{
	cmp::Ordering,
	collections::{BTreeMap, HashMap, hash_set::HashSet},
};

use serde_json;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	Error, NoteFetchResponse, PayloadLevel, Result,
	access::{self, SharedSpaceGrantKey},
	progressive_search::types::{
		HitItem, SearchDetailsError, SearchDetailsResult, SearchIndexItem, SearchSession,
		SearchSessionItemRecord, SearchTimelineGroup, SearchTimelineResponse,
	},
	structured_fields::StructuredFields,
};
use elf_config::Config;
use elf_storage::models::MemoryNote;

pub(super) struct SearchDetailsBuildArgs<'a> {
	pub(super) session_items_by_note_id: &'a HashMap<Uuid, SearchSessionItemRecord>,
	pub(super) notes_by_id: &'a HashMap<Uuid, MemoryNote>,
	pub(super) structured_by_note: &'a HashMap<Uuid, StructuredFields>,
	pub(super) session: &'a SearchSession,
	pub(super) shared_grants: &'a HashSet<SharedSpaceGrantKey>,
	pub(super) allowed_scopes: &'a [String],
	pub(super) now: OffsetDateTime,
	pub(super) record_hits_enabled: bool,
	pub(super) payload_level: PayloadLevel,
	pub(super) max_note_chars: usize,
}

pub(super) fn build_search_details_results(
	requested_note_ids: Vec<Uuid>,
	args: SearchDetailsBuildArgs<'_>,
) -> (Vec<SearchDetailsResult>, Vec<HitItem>) {
	let mut results = Vec::with_capacity(requested_note_ids.len());
	let mut hits = Vec::new();
	let mut hit_seen = HashSet::new();

	for note_id in requested_note_ids {
		let Some(session_item) = args.session_items_by_note_id.get(&note_id) else {
			results.push(SearchDetailsResult {
				note_id,
				note: None,
				error: Some(SearchDetailsError {
					code: "NOT_IN_SESSION".to_string(),
					message: "Requested note_id is not present in the search session.".to_string(),
				}),
			});

			continue;
		};
		let Some(note) = args.notes_by_id.get(&note_id) else {
			results.push(SearchDetailsResult {
				note_id,
				note: None,
				error: Some(SearchDetailsError {
					code: "NOTE_NOT_FOUND".to_string(),
					message: "Note not found.".to_string(),
				}),
			});

			continue;
		};
		let error = validate_note_access(
			note,
			args.session,
			args.allowed_scopes,
			args.shared_grants,
			args.now,
		);

		if let Some(error) = error {
			results.push(SearchDetailsResult { note_id, note: None, error: Some(error) });

			continue;
		}

		let structured = if args.payload_level == PayloadLevel::L0 {
			None
		} else {
			args.structured_by_note.get(&note.note_id).cloned()
		};
		let note_text = apply_payload_level_to_search_details_text(
			note.text.as_str(),
			structured.as_ref(),
			args.payload_level,
			args.max_note_chars,
		);
		let source_ref = if args.payload_level == PayloadLevel::L2 {
			note.source_ref.clone()
		} else {
			serde_json::json!({})
		};
		let note_response = NoteFetchResponse {
			note_id: note.note_id,
			tenant_id: note.tenant_id.clone(),
			project_id: note.project_id.clone(),
			agent_id: note.agent_id.clone(),
			scope: note.scope.clone(),
			r#type: note.r#type.clone(),
			key: note.key.clone(),
			text: note_text,
			importance: note.importance,
			confidence: note.confidence,
			status: note.status.clone(),
			updated_at: note.updated_at,
			expires_at: note.expires_at,
			source_ref,
			structured,
		};

		results.push(SearchDetailsResult { note_id, note: Some(note_response), error: None });

		if args.record_hits_enabled && hit_seen.insert(note_id) {
			hits.push(HitItem {
				note_id,
				chunk_id: session_item.chunk_id,
				rank: session_item.rank,
				final_score: session_item.final_score,
			});
		}
	}

	(results, hits)
}

pub(super) fn build_timeline_by_day(
	search_session_id: Uuid,
	expires_at: OffsetDateTime,
	items: &[SearchSessionItemRecord],
) -> Result<SearchTimelineResponse> {
	let mut grouped: BTreeMap<String, Vec<SearchIndexItem>> = BTreeMap::new();

	for item in items {
		let date = item.updated_at.date().to_string();

		grouped.entry(date).or_default().push(item.to_index_item());
	}

	let mut groups = Vec::with_capacity(grouped.len());

	for (date, mut items) in grouped.into_iter().rev() {
		items.sort_by(|a, b| {
			b.updated_at
				.cmp(&a.updated_at)
				.then_with(|| b.final_score.partial_cmp(&a.final_score).unwrap_or(Ordering::Equal))
		});
		groups.push(SearchTimelineGroup { date, items });
	}

	Ok(SearchTimelineResponse { search_session_id, expires_at, groups })
}

pub(super) fn build_summary(raw: &str, max_chars: usize) -> String {
	let normalized = normalize_whitespace(raw);

	truncate_chars(&normalized, max_chars)
}

pub(super) fn resolve_read_scopes(cfg: &Config, profile: &str) -> Result<Vec<String>> {
	match profile {
		"private_only" => Ok(cfg.scopes.read_profiles.private_only.clone()),
		"private_plus_project" => Ok(cfg.scopes.read_profiles.private_plus_project.clone()),
		"all_scopes" => Ok(cfg.scopes.read_profiles.all_scopes.clone()),
		_ => Err(Error::InvalidRequest { message: "Unknown read_profile.".to_string() }),
	}
}

pub(super) fn validate_search_session_access(
	session: &SearchSession,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
) -> Result<()> {
	if session.tenant_id != tenant_id
		|| session.project_id != project_id
		|| session.agent_id != agent_id
	{
		return Err(Error::InvalidRequest { message: "Unknown search_session_id.".to_string() });
	}

	Ok(())
}

fn apply_payload_level_to_search_details_text(
	raw_text: &str,
	structured: Option<&StructuredFields>,
	payload_level: PayloadLevel,
	max_note_chars: usize,
) -> String {
	match payload_level {
		PayloadLevel::L0 => build_summary(raw_text, max_note_chars),
		PayloadLevel::L1 => {
			let candidate_text = structured
				.and_then(|item| item.summary.as_deref())
				.filter(|summary| !summary.trim().is_empty())
				.unwrap_or(raw_text);

			build_summary(candidate_text, max_note_chars)
		},
		PayloadLevel::L2 => raw_text.to_string(),
	}
}

fn normalize_whitespace(raw: &str) -> String {
	let mut out = String::with_capacity(raw.len());
	let mut prev_space = false;

	for ch in raw.chars() {
		if ch.is_whitespace() {
			if !prev_space {
				out.push(' ');

				prev_space = true;
			}

			continue;
		}

		out.push(ch);

		prev_space = false;
	}

	out.trim().to_string()
}

fn truncate_chars(raw: &str, max_chars: usize) -> String {
	if raw.chars().count() <= max_chars {
		return raw.to_string();
	}

	const TRUNCATION_MARKER: &str = "...";

	let marker_chars = TRUNCATION_MARKER.chars().count();

	if max_chars <= marker_chars {
		return TRUNCATION_MARKER.chars().take(max_chars).collect();
	}

	let truncated_chars = max_chars - marker_chars;
	let mut out = String::with_capacity(max_chars);

	for (idx, ch) in raw.chars().enumerate() {
		if idx >= truncated_chars {
			break;
		}

		out.push(ch);
	}

	out.push_str(TRUNCATION_MARKER);

	out
}

fn validate_note_access(
	note: &MemoryNote,
	session: &SearchSession,
	allowed_scopes: &[String],
	shared_grants: &HashSet<SharedSpaceGrantKey>,
	now: OffsetDateTime,
) -> Option<SearchDetailsError> {
	if note.status != "active" {
		return Some(SearchDetailsError {
			code: "NOTE_INACTIVE".to_string(),
			message: "Note is not active.".to_string(),
		});
	}
	if note.expires_at.map(|ts| ts <= now).unwrap_or(false) {
		return Some(SearchDetailsError {
			code: "NOTE_EXPIRED".to_string(),
			message: "Note is expired.".to_string(),
		});
	}
	if !allowed_scopes.iter().any(|scope| scope == &note.scope) {
		return Some(SearchDetailsError {
			code: "SCOPE_DENIED".to_string(),
			message: "Note scope is not allowed for this read_profile.".to_string(),
		});
	}
	if !access::note_read_allowed(
		note,
		session.agent_id.as_str(),
		allowed_scopes,
		shared_grants,
		now,
	) {
		return Some(SearchDetailsError {
			code: "SCOPE_DENIED".to_string(),
			message: "Note scope is not allowed for this read_profile.".to_string(),
		});
	}

	None
}
