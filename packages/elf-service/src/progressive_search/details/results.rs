use std::collections::{HashMap, hash_set::HashSet};

use serde_json;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	NoteFetchResponse, PayloadLevel,
	access::SharedSpaceGrantKey,
	progressive_search::{
		details::{access, text},
		types::{
			HitItem, SearchDetailsError, SearchDetailsResult, SearchSession,
			SearchSessionItemRecord,
		},
	},
	structured_fields::StructuredFields,
};
use elf_storage::models::MemoryNote;

pub(crate) struct SearchDetailsBuildArgs<'a> {
	pub(crate) session_items_by_note_id: &'a HashMap<Uuid, SearchSessionItemRecord>,
	pub(crate) notes_by_id: &'a HashMap<Uuid, MemoryNote>,
	pub(crate) structured_by_note: &'a HashMap<Uuid, StructuredFields>,
	pub(crate) session: &'a SearchSession,
	pub(crate) shared_grants: &'a HashSet<SharedSpaceGrantKey>,
	pub(crate) allowed_scopes: &'a [String],
	pub(crate) now: OffsetDateTime,
	pub(crate) record_hits_enabled: bool,
	pub(crate) payload_level: PayloadLevel,
	pub(crate) max_note_chars: usize,
}

pub(crate) fn build_search_details_results(
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
		let error = access::validate_note_access(
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
		let note_text = text::apply_payload_level_to_search_details_text(
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
