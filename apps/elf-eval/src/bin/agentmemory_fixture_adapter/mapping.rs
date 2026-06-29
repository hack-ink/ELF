use std::collections::HashMap;

use serde_json::Value;

use super::{
	DEFAULT_CONFIDENCE, DEFAULT_IMPORTANCE, FIXTURE_RESOLVER,
	types::{
		AgentmemoryMemory, AgentmemoryObservation, AgentmemoryRetrievalCase, AgentmemorySession,
		BaselineQuery, DocCandidate, DocsPutCandidate, ElfNoteCandidate, FixtureContext,
		NoteCandidate,
	},
	util::{clean_string, map_note_type, observation_timestamp, score_or_default, stable_uuid},
};

pub(super) fn doc_candidate(
	session: &AgentmemorySession,
	observation: &AgentmemoryObservation,
	ctx: &FixtureContext,
) -> std::result::Result<DocCandidate, &'static str> {
	let text = observation.text.trim();

	if text.is_empty() {
		return Err("empty_text");
	}

	let Some(ts) = observation_timestamp(session, observation, ctx) else {
		return Err("missing_or_invalid_timestamp");
	};
	let candidate_id = stable_uuid(
		"observation",
		&[
			ctx.fixture_id.as_str(),
			session.session_id.as_str(),
			observation.observation_id.as_str(),
		],
	);
	let role = clean_string(observation.role.as_deref())
		.or_else(|| clean_string(observation.kind.as_deref()))
		.unwrap_or_else(|| "observation".to_string());
	let title = format!("agentmemory observation {}", observation.observation_id);
	let source_ref = serde_json::json!({
		"schema": "doc_source_ref/v1",
		"doc_type": "chat",
		"ts": ts,
		"thread_id": session.session_id,
		"role": role,
		"message_id": observation.observation_id,
		"agentmemory_fixture_id": ctx.fixture_id,
		"agentmemory_source_system": ctx.source_system,
		"agentmemory_observation_kind": clean_string(observation.kind.as_deref()),
		"agent": clean_string(session.agent.as_deref()),
		"project": clean_string(session.project.as_deref()),
	});

	Ok(DocCandidate {
		candidate_id,
		scope: ctx.scope.clone(),
		session_id: session.session_id.clone(),
		source_observation_id: observation.observation_id.clone(),
		docs_put: DocsPutCandidate {
			scope: ctx.scope.clone(),
			doc_type: "chat",
			title,
			source_ref,
			content: observation.text.clone(),
		},
		source_metadata: observation.metadata.clone(),
	})
}

pub(super) fn note_candidate(
	session: &AgentmemorySession,
	memory: &AgentmemoryMemory,
	ctx: &FixtureContext,
) -> std::result::Result<NoteCandidate, &'static str> {
	let text = memory.text.trim();

	if text.is_empty() {
		return Err("empty_text");
	}
	if text.chars().count() > ctx.max_note_chars {
		return Err("note_text_too_long");
	}

	let Some(note_type) = memory.kind.as_deref().and_then(map_note_type) else {
		return Err("unsupported_memory_kind");
	};
	let Some(importance) = score_or_default(memory.importance, DEFAULT_IMPORTANCE) else {
		return Err("invalid_importance");
	};
	let Some(confidence) = score_or_default(memory.confidence, DEFAULT_CONFIDENCE) else {
		return Err("invalid_confidence");
	};
	let candidate_id = stable_uuid(
		"memory",
		&[ctx.fixture_id.as_str(), session.session_id.as_str(), memory.memory_id.as_str()],
	);
	let source_ref = note_source_ref(session, memory, ctx);

	Ok(NoteCandidate {
		candidate_id,
		scope: ctx.scope.clone(),
		session_id: session.session_id.clone(),
		source_memory_id: memory.memory_id.clone(),
		source_observation_ids: memory.source_observation_ids.clone(),
		notes_ingest_item: ElfNoteCandidate {
			note_type: note_type.to_string(),
			key: clean_string(memory.key.as_deref()),
			text: memory.text.clone(),
			importance,
			confidence,
			ttl_days: memory.ttl_days.filter(|days| *days > 0),
			source_ref,
		},
		source_metadata: memory.metadata.clone(),
	})
}

fn note_source_ref(
	session: &AgentmemorySession,
	memory: &AgentmemoryMemory,
	ctx: &FixtureContext,
) -> Value {
	serde_json::json!({
		"schema": "source_ref/v1",
		"resolver": FIXTURE_RESOLVER,
		"ref": {
			"fixture_id": ctx.fixture_id,
			"session_id": session.session_id,
			"memory_id": memory.memory_id,
			"observation_ids": memory.source_observation_ids,
		},
		"state": {
			"source_system": ctx.source_system,
			"source_version": ctx.source_version,
			"exported_at": ctx.exported_at,
			"session_started_at": session.started_at,
			"session_ended_at": session.ended_at,
			"memory_created_at": memory.created_at,
			"memory_updated_at": memory.updated_at,
		},
		"locator": {
			"memory_id": memory.memory_id,
			"observation_ids": memory.source_observation_ids,
		},
		"hints": {
			"agent": session.agent,
			"project": session.project,
			"origin_kind": memory.kind,
		},
	})
}

pub(super) fn baseline_query(
	session: &AgentmemorySession,
	case: &AgentmemoryRetrievalCase,
	memory_map: &HashMap<String, NoteCandidate>,
) -> Option<BaselineQuery> {
	if case.query.trim().is_empty() || case.expected_memory_ids.is_empty() {
		return None;
	}

	let expected: Vec<&NoteCandidate> =
		case.expected_memory_ids.iter().filter_map(|id| memory_map.get(id)).collect();

	if expected.is_empty() {
		return None;
	}

	Some(BaselineQuery {
		query_id: case.query_id.clone(),
		session_id: session.session_id.clone(),
		query: case.query.clone(),
		expected_source_memory_ids: expected
			.iter()
			.map(|candidate| candidate.source_memory_id.clone())
			.collect(),
		expected_candidate_ids: expected.iter().map(|candidate| candidate.candidate_id).collect(),
		expected_keys: expected
			.iter()
			.filter_map(|candidate| candidate.notes_ingest_item.key.clone())
			.collect(),
		agentmemory_results: case.agentmemory_results.clone(),
		source_metadata: case.metadata.clone(),
	})
}
