mod excerpts;
mod non_english;
mod put;
mod search;
mod source_ref;

pub(in crate::docs) use self::{
	excerpts::{excerpt_level_max, resolve_doc_chunking_profile, validate_docs_excerpts_get},
	put::validate_docs_put,
	search::validate_docs_search_l0,
};

use crate::docs::{
	DEFAULT_DOC_MAX_BYTES, DEFAULT_L0_MAX_BYTES, DEFAULT_L1_MAX_BYTES, DEFAULT_L2_MAX_BYTES,
	DEFAULT_MAX_CHUNKS_PER_DOC, DOC_STATUSES, DocChunkingProfile, DocType, DocsPutRequest,
	DocsSearchL0Filters, DocsSearchL0FiltersParsed, DocsSearchL0RangesParsed, DocsSearchL0Request,
	DocsSparseMode, Error, Map, OffsetDateTime, Result, Rfc3339, SOURCE_LIBRARY_FIELD_KEYS,
	SOURCE_LIBRARY_KINDS, SOURCE_LIBRARY_TRUST_LABELS, TextQuoteSelector, ValidatedDocsPut, Value,
	english_gate, writegate,
};
