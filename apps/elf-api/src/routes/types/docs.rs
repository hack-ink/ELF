use super::*;

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct DocsPutBody {
	pub(in crate::routes) scope: String,
	pub(in crate::routes) doc_type: Option<DocType>,
	pub(in crate::routes) title: Option<String>,
	#[serde(default)]
	pub(in crate::routes) source_ref: Value,

	pub(in crate::routes) write_policy: Option<WritePolicy>,
	pub(in crate::routes) content: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct DocsSearchL0Body {
	pub(in crate::routes) query: String,
	pub(in crate::routes) scope: Option<String>,
	pub(in crate::routes) status: Option<String>,
	pub(in crate::routes) doc_type: Option<DocType>,
	pub(in crate::routes) sparse_mode: Option<String>,
	pub(in crate::routes) domain: Option<String>,
	pub(in crate::routes) repo: Option<String>,
	pub(in crate::routes) agent_id: Option<String>,
	pub(in crate::routes) thread_id: Option<String>,
	pub(in crate::routes) updated_after: Option<String>,
	pub(in crate::routes) updated_before: Option<String>,
	pub(in crate::routes) ts_gte: Option<String>,
	pub(in crate::routes) ts_lte: Option<String>,
	pub(in crate::routes) top_k: Option<u32>,
	pub(in crate::routes) candidate_k: Option<u32>,
	pub(in crate::routes) explain: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct DocsExcerptsGetBody {
	pub(in crate::routes) doc_id: Uuid,
	pub(in crate::routes) level: String,
	pub(in crate::routes) chunk_id: Option<Uuid>,
	pub(in crate::routes) quote: Option<TextQuoteSelector>,
	pub(in crate::routes) position: Option<TextPositionSelector>,
	pub(in crate::routes) explain: Option<bool>,
}
