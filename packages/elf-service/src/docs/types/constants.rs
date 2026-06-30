pub(in crate::docs) const MAX_TOP_K: u32 = 32;
pub(in crate::docs) const MAX_CANDIDATE_K: u32 = 1_024;
pub(in crate::docs) const DEFAULT_DOC_MAX_BYTES: usize = 4 * 1_024 * 1_024;
pub(in crate::docs) const DEFAULT_MAX_CHUNKS_PER_DOC: usize = 4_096;
pub(in crate::docs) const DEFAULT_L0_MAX_BYTES: usize = 256;
pub(in crate::docs) const DEFAULT_L1_MAX_BYTES: usize = 8 * 1_024;
pub(in crate::docs) const DEFAULT_L2_MAX_BYTES: usize = 32 * 1_024;
pub(in crate::docs) const DOC_RETRIEVAL_TRAJECTORY_SCHEMA_V1: &str = "doc_retrieval_trajectory/v1";
pub(in crate::docs) const DOC_SOURCE_REF_SCHEMA_V1: &str = "source_ref/v1";
pub(in crate::docs) const DOC_SOURCE_REF_RESOLVER_V1: &str = "elf_doc_ext/v1";
pub(in crate::docs) const DOC_SOURCE_CAPTURE_SCHEMA_V1: &str = "doc_source_capture/v1";
pub(in crate::docs) const DOC_SOURCE_SPAN_SCHEMA_V1: &str = "doc_source_span/v1";
pub(in crate::docs) const DOC_STATUSES: [&str; 2] = ["active", "deleted"];
pub(in crate::docs) const SOURCE_LIBRARY_FIELD_KEYS: [&str; 9] = [
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
pub(in crate::docs) const SOURCE_LIBRARY_KINDS: [&str; 7] =
	["article", "social_thread", "pdf", "text_export", "repo_file", "chat_excerpt", "web_page"];
pub(in crate::docs) const SOURCE_LIBRARY_TRUST_LABELS: [&str; 5] =
	["trusted", "user_captured", "public_web", "third_party", "unverified"];
