use super::*;

pub(super) const MAX_TOP_K: u32 = 32;
pub(super) const MAX_CANDIDATE_K: u32 = 1_024;
pub(super) const DEFAULT_DOC_MAX_BYTES: usize = 4 * 1_024 * 1_024;
pub(super) const DEFAULT_MAX_CHUNKS_PER_DOC: usize = 4_096;
pub(super) const DEFAULT_L0_MAX_BYTES: usize = 256;
pub(super) const DEFAULT_L1_MAX_BYTES: usize = 8 * 1_024;
pub(super) const DEFAULT_L2_MAX_BYTES: usize = 32 * 1_024;
pub(super) const DOC_RETRIEVAL_TRAJECTORY_SCHEMA_V1: &str = "doc_retrieval_trajectory/v1";
pub(super) const DOC_SOURCE_REF_SCHEMA_V1: &str = "source_ref/v1";
pub(super) const DOC_SOURCE_REF_RESOLVER_V1: &str = "elf_doc_ext/v1";
pub(super) const DOC_SOURCE_CAPTURE_SCHEMA_V1: &str = "doc_source_capture/v1";
pub(super) const DOC_SOURCE_SPAN_SCHEMA_V1: &str = "doc_source_span/v1";
pub(super) const DOC_STATUSES: [&str; 2] = ["active", "deleted"];
pub(super) const SOURCE_LIBRARY_FIELD_KEYS: [&str; 9] = [
	"source_kind",
	"canonical_uri",
	"captured_at",
	"source_created_at",
	"trust_label",
	"author",
	"handle",
	"excerpt_locator",
	"source_content_hash",
];
pub(super) const SOURCE_LIBRARY_KINDS: [&str; 7] =
	["article", "social_thread", "pdf", "text_export", "repo_file", "chat_excerpt", "web_page"];
pub(super) const SOURCE_LIBRARY_TRUST_LABELS: [&str; 5] =
	["trusted", "user_captured", "public_web", "third_party", "unverified"];

pub(super) struct SourceCaptureSummaryInput<'a> {
	pub(super) doc_id: Uuid,
	pub(super) source_ref: &'a Map<String, Value>,
	pub(super) doc_type: DocType,
	pub(super) scope: &'a str,
	pub(super) title: Option<&'a str>,
	pub(super) content_hash: &'a str,
	pub(super) raw_content_hash: &'a str,
	pub(super) now: OffsetDateTime,
	pub(super) chunks: &'a [DocChunk],
	pub(super) write_policy_audit: Option<&'a WritePolicyAudit>,
}

#[derive(Clone, Copy)]
pub(super) struct DocExcerptMatch {
	pub(super) selector_kind: ExcerptsSelectorKind,
	pub(super) match_start_offset: usize,
	pub(super) match_end_offset: usize,
}

pub(super) struct DocExcerptRange {
	pub(super) selector_kind: ExcerptsSelectorKind,
	pub(super) match_start_offset: usize,
	pub(super) match_end_offset: usize,
	pub(super) start_offset: usize,
	pub(super) end_offset: usize,
}

pub(super) struct DocTrajectoryBuilder {
	pub(super) explain: bool,
	pub(super) stages: Vec<DocRetrievalTrajectoryStage>,
	pub(super) stage_order: u32,
}
impl DocTrajectoryBuilder {
	pub(super) fn new(explain: bool) -> Self {
		Self { explain, stages: Vec::new(), stage_order: 0 }
	}

	pub(super) fn push(&mut self, stage_name: &str, stats: Value) {
		if !self.explain {
			return;
		}

		self.stages.push(DocRetrievalTrajectoryStage {
			stage_order: self.stage_order,
			stage_name: stage_name.to_string(),
			stats,
		});

		self.stage_order += 1;
	}

	pub(super) fn into_trajectory(self) -> Option<DocRetrievalTrajectory> {
		if !self.explain {
			return None;
		}

		Some(DocRetrievalTrajectory {
			schema: DOC_RETRIEVAL_TRAJECTORY_SCHEMA_V1.to_string(),
			stages: self.stages,
		})
	}
}

#[derive(Clone, Debug)]
pub(super) struct DocsSearchL0Filters {
	pub(super) scope: Option<String>,
	pub(super) status: String,
	pub(super) doc_type: Option<DocType>,
	pub(super) sparse_mode: DocsSparseMode,
	pub(super) domain: Option<String>,
	pub(super) repo: Option<String>,
	pub(super) agent_id: Option<String>,
	pub(super) thread_id: Option<String>,
	pub(super) updated_after: Option<OffsetDateTime>,
	pub(super) updated_before: Option<OffsetDateTime>,
	pub(super) ts_gte: Option<OffsetDateTime>,
	pub(super) ts_lte: Option<OffsetDateTime>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct DocChunkingProfile {
	pub(super) max_tokens: usize,
	pub(super) overlap_tokens: usize,
	pub(super) max_chunks: usize,
}

#[derive(Clone, Debug)]
pub(super) struct ByteChunk {
	pub(super) chunk_id: Uuid,
	pub(super) start_offset: usize,
	pub(super) end_offset: usize,
	pub(super) text: String,
}

#[derive(Debug)]
pub(super) struct ValidatedDocsPut {
	pub(super) doc_type: DocType,
	pub(super) content: String,
	pub(super) write_policy_audit: Option<WritePolicyAudit>,
}

#[derive(Clone, Debug, FromRow)]
pub(super) struct DocSearchRow {
	pub(super) chunk_id: Uuid,
	pub(super) doc_id: Uuid,
	pub(super) scope: String,
	pub(super) doc_type: String,
	pub(super) project_id: String,
	pub(super) agent_id: String,
	pub(super) updated_at: OffsetDateTime,
	pub(super) content_hash: String,
	pub(super) chunk_hash: String,
	pub(super) start_offset: i32,
	pub(super) end_offset: i32,
	pub(super) chunk_text: String,
}

pub(super) struct DocsSearchL0Prepared {
	pub(super) top_k: u32,
	pub(super) candidate_k: u32,
	pub(super) sparse_mode: DocsSparseMode,
	pub(super) sparse_enabled: bool,
	pub(super) now: OffsetDateTime,
	pub(super) trajectory: DocTrajectoryBuilder,
	pub(super) allowed_scopes: Vec<String>,
	pub(super) shared_grants: HashSet<SharedSpaceGrantKey>,
	pub(super) filter: Filter,
	pub(super) vector: Vec<f32>,
	pub(super) status: String,
}

#[derive(Debug)]
pub(super) struct DocsSearchL0FiltersParsed {
	pub(super) scope: Option<String>,
	pub(super) status: String,
	pub(super) doc_type: Option<DocType>,
	pub(super) sparse_mode: DocsSparseMode,
	pub(super) domain: Option<String>,
	pub(super) repo: Option<String>,
	pub(super) agent_id: Option<String>,
	pub(super) thread_id: Option<String>,
}

#[derive(Debug)]
pub(super) struct DocsSearchL0RangesParsed {
	pub(super) updated_after: Option<OffsetDateTime>,
	pub(super) updated_before: Option<OffsetDateTime>,
	pub(super) ts_gte: Option<OffsetDateTime>,
	pub(super) ts_lte: Option<OffsetDateTime>,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum DocsSparseMode {
	Auto,
	On,
	Off,
}
impl DocsSparseMode {
	pub(super) fn as_str(self) -> &'static str {
		match self {
			Self::Auto => "auto",
			Self::On => "on",
			Self::Off => "off",
		}
	}
}

#[derive(Clone, Copy)]
pub(super) enum ExcerptsSelectorKind {
	ChunkId,
	Quote,
	Position,
}
impl ExcerptsSelectorKind {
	pub(super) fn as_str(&self) -> &'static str {
		match self {
			Self::ChunkId => "chunk_id",
			Self::Quote => "quote",
			Self::Position => "position",
		}
	}

	pub(super) fn span_kind(&self) -> &'static str {
		match self {
			Self::ChunkId => "captured",
			Self::Quote => "quote",
			Self::Position => "position",
		}
	}
}
