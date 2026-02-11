use std::{
	collections::{HashMap, hash_map::DefaultHasher},
	hash::{Hash, Hasher},
};

use serde::de::DeserializeOwned;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	Error, Result,
	search::{RerankCacheCandidate, RerankCachePayload},
};

const EXPANSION_CACHE_SCHEMA_VERSION: i32 = 1;
const RERANK_CACHE_SCHEMA_VERSION: i32 = 1;

pub fn decode_json<T>(value: Value, label: &str) -> Result<T>
where
	T: DeserializeOwned,
{
	serde_json::from_value(value)
		.map_err(|err| Error::Storage { message: format!("Invalid {label} value: {err}") })
}

pub fn hash_query(query: &str) -> String {
	let mut hasher = DefaultHasher::new();

	Hash::hash(query, &mut hasher);

	format!("{:x}", hasher.finish())
}

pub fn hash_cache_key(payload: &Value) -> Result<String> {
	let raw = serde_json::to_vec(payload).map_err(|err| Error::Storage {
		message: format!("Failed to encode cache key payload: {err}"),
	})?;

	Ok(blake3::hash(&raw).to_hex().to_string())
}

pub fn cache_key_prefix(key: &str) -> &str {
	let len = key.len().min(12);

	&key[..len]
}

pub fn build_expansion_cache_key(
	query: &str,
	max_queries: u32,
	include_original: bool,
	provider_id: &str,
	model: &str,
	temperature: f32,
) -> Result<String> {
	let payload = serde_json::json!({
		"kind": "expansion",
		"schema_version": EXPANSION_CACHE_SCHEMA_VERSION,
		"query": query.trim(),
		"provider_id": provider_id,
		"model": model,
		"temperature": temperature,
		"max_queries": max_queries,
		"include_original": include_original,
	});

	hash_cache_key(&payload)
}

pub fn build_rerank_cache_key(
	query: &str,
	provider_id: &str,
	model: &str,
	candidates: &[(Uuid, OffsetDateTime)],
) -> Result<String> {
	let signature: Vec<Value> = candidates
		.iter()
		.map(|(chunk_id, updated_at)| {
			serde_json::json!({
				"chunk_id": chunk_id,
				"updated_at": updated_at,
			})
		})
		.collect();
	let payload = serde_json::json!({
		"kind": "rerank",
		"schema_version": RERANK_CACHE_SCHEMA_VERSION,
		"query": query.trim(),
		"provider_id": provider_id,
		"model": model,
		"candidates": signature,
	});

	hash_cache_key(&payload)
}

pub fn build_cached_scores(
	payload: &RerankCachePayload,
	candidates: &[RerankCacheCandidate],
) -> Option<Vec<f32>> {
	if payload.items.len() != candidates.len() {
		return None;
	}

	let mut map = HashMap::new();

	for item in &payload.items {
		let key = (item.chunk_id, item.updated_at.unix_timestamp(), item.updated_at.nanosecond());

		map.insert(key, item.score);
	}

	let mut out = Vec::with_capacity(candidates.len());

	for candidate in candidates {
		let key = (
			candidate.chunk_id,
			candidate.updated_at.unix_timestamp(),
			candidate.updated_at.nanosecond(),
		);
		let score = map.get(&key)?;

		out.push(*score);
	}

	Some(out)
}
