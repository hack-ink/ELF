use uuid::Uuid;

#[derive(Clone, Copy)]
pub(super) struct DiversityPick {
	pub(super) remaining_pos: usize,
	pub(super) mmr_score: f32,
	pub(super) nearest_note_id: Option<Uuid>,
	pub(super) similarity: Option<f32>,
	pub(super) missing_embedding: bool,
	pub(super) retrieval_rank: u32,
}
impl DiversityPick {
	pub(super) fn better_than(self, other: &Self) -> bool {
		self.mmr_score > other.mmr_score
			|| (self.mmr_score == other.mmr_score && self.retrieval_rank < other.retrieval_rank)
	}
}
