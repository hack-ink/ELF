use std::{fs, path::Path};

use color_eyre::{Result, eyre};
use uuid::Uuid;

use crate::app::{
	Args,
	types::{EvalDataset, EvalDefaults, EvalQuery, ExpectedKind, MergedQuery},
};
use elf_config::Config;
use elf_service::SearchRequest;

pub(super) fn load_dataset(path: &Path) -> Result<EvalDataset> {
	let raw = fs::read_to_string(path)?;
	let dataset: EvalDataset = serde_json::from_str(&raw)?;

	if dataset.queries.is_empty() {
		return Err(eyre::eyre!("Dataset must include at least one query."));
	}

	Ok(dataset)
}

pub(super) fn merge_query(
	defaults: &EvalDefaults,
	query: &EvalQuery,
	args: &Args,
	cfg: &Config,
	index: usize,
) -> Result<MergedQuery> {
	let expected_kind =
		resolve_expected_mode(index, &query.expected_note_ids, &query.expected_keys)?;
	let tenant_id = query
		.tenant_id
		.clone()
		.or_else(|| defaults.tenant_id.clone())
		.ok_or_else(|| eyre::eyre!("tenant_id is required for query at index {index}."))?;
	let project_id = query
		.project_id
		.clone()
		.or_else(|| defaults.project_id.clone())
		.ok_or_else(|| eyre::eyre!("project_id is required for query at index {index}."))?;
	let agent_id = query
		.agent_id
		.clone()
		.or_else(|| defaults.agent_id.clone())
		.ok_or_else(|| eyre::eyre!("agent_id is required for query at index {index}."))?;
	let read_profile = query
		.read_profile
		.clone()
		.or_else(|| defaults.read_profile.clone())
		.ok_or_else(|| eyre::eyre!("read_profile is required for query at index {index}."))?;
	let top_k = args.top_k.or(query.top_k).or(defaults.top_k).unwrap_or(cfg.memory.top_k).max(1);
	let candidate_k = args
		.candidate_k
		.or(query.candidate_k)
		.or(defaults.candidate_k)
		.unwrap_or(cfg.memory.candidate_k)
		.max(top_k);
	let id = query.id.clone().unwrap_or_else(|| format!("query-{index}"));
	let ranking = query.ranking.clone().or_else(|| defaults.ranking.clone());

	Ok(MergedQuery {
		id,
		query: query.query.clone(),
		expected_note_ids: query.expected_note_ids.clone(),
		expected_keys: query.expected_keys.clone(),
		expected_kind,
		request: SearchRequest {
			tenant_id,
			project_id,
			agent_id,
			token_id: None,
			read_profile,
			payload_level: Default::default(),
			query: query.query.clone(),
			top_k: Some(top_k),
			candidate_k: Some(candidate_k),
			filter: None,
			record_hits: Some(false),
			ranking,
		},
	})
}

pub(super) fn resolve_expected_mode(
	index: usize,
	note_ids: &[Uuid],
	keys: &[String],
) -> Result<ExpectedKind> {
	let has_note_ids = !note_ids.is_empty();
	let has_keys = !keys.is_empty();

	match (has_note_ids, has_keys) {
		(true, false) => Ok(ExpectedKind::NoteId),
		(false, true) => Ok(ExpectedKind::Key),
		(true, true) => Err(eyre::eyre!(
			"Query at index {index} must define exactly one expectation mode: expected_note_ids or expected_keys."
		)),
		(false, false) => Err(eyre::eyre!(
			"Query at index {index} must include at least one expected_note_ids or expected_keys."
		)),
	}
}
