use crate::search::{Deserialize, OffsetDateTime, Serialize, Uuid, Value};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(in crate::search) struct ExpansionCachePayload {
	pub(in crate::search) queries: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(in crate::search) struct ExpansionOutput {
	pub(in crate::search) queries: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(in crate::search) struct RerankCacheItem {
	pub(in crate::search) chunk_id: Uuid,
	pub(in crate::search) updated_at: OffsetDateTime,
	pub(in crate::search) score: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(in crate::search) struct RerankCachePayload {
	pub(in crate::search) items: Vec<RerankCacheItem>,
}

#[derive(Clone, Debug)]
pub(in crate::search) struct CachePayload {
	pub(in crate::search) value: Value,
	pub(in crate::search) size_bytes: usize,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::search) enum CacheKind {
	Expansion,
	Rerank,
}
impl CacheKind {
	pub(in crate::search) fn as_str(self) -> &'static str {
		match self {
			Self::Expansion => "expansion",
			Self::Rerank => "rerank",
		}
	}
}
