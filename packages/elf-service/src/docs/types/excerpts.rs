#[derive(Clone, Copy)]
pub(in crate::docs) struct DocExcerptMatch {
	pub(in crate::docs) selector_kind: ExcerptsSelectorKind,
	pub(in crate::docs) match_start_offset: usize,
	pub(in crate::docs) match_end_offset: usize,
}

pub(in crate::docs) struct DocExcerptRange {
	pub(in crate::docs) selector_kind: ExcerptsSelectorKind,
	pub(in crate::docs) match_start_offset: usize,
	pub(in crate::docs) match_end_offset: usize,
	pub(in crate::docs) start_offset: usize,
	pub(in crate::docs) end_offset: usize,
}

#[derive(Clone, Copy)]
pub(in crate::docs) enum ExcerptsSelectorKind {
	ChunkId,
	Quote,
	Position,
}
impl ExcerptsSelectorKind {
	pub(in crate::docs) fn as_str(&self) -> &'static str {
		match self {
			Self::ChunkId => "chunk_id",
			Self::Quote => "quote",
			Self::Position => "position",
		}
	}

	pub(in crate::docs) fn span_kind(&self) -> &'static str {
		match self {
			Self::ChunkId => "captured",
			Self::Quote => "quote",
			Self::Position => "position",
		}
	}
}
