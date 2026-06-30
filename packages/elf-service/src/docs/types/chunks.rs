use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
pub(in crate::docs) struct DocChunkingProfile {
	pub(in crate::docs) max_tokens: usize,
	pub(in crate::docs) overlap_tokens: usize,
	pub(in crate::docs) max_chunks: usize,
}

#[derive(Clone, Debug)]
pub(in crate::docs) struct ByteChunk {
	pub(in crate::docs) chunk_id: Uuid,
	pub(in crate::docs) start_offset: usize,
	pub(in crate::docs) end_offset: usize,
	pub(in crate::docs) text: String,
}
