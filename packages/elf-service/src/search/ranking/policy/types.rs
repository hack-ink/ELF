#[derive(Clone, Copy, Debug)]
pub enum NormalizationKind {
	Rank,
}
impl NormalizationKind {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Rank => "rank",
		}
	}
}

#[derive(Clone, Debug)]
pub struct BlendSegment {
	pub max_retrieval_rank: u32,
	pub retrieval_weight: f32,
}

#[derive(Clone, Debug)]
pub struct ResolvedBlendPolicy {
	pub enabled: bool,
	pub rerank_normalization: NormalizationKind,
	pub retrieval_normalization: NormalizationKind,
	pub segments: Vec<BlendSegment>,
}

#[derive(Clone, Debug)]
pub struct ResolvedDiversityPolicy {
	pub enabled: bool,
	pub sim_threshold: f32,
	pub mmr_lambda: f32,
	pub max_skips: u32,
}

#[derive(Clone, Debug)]
pub struct ResolvedRetrievalSourcesPolicy {
	pub fusion_weight: f32,
	pub structured_field_weight: f32,
	pub recursive_weight: f32,
	pub fusion_priority: u32,
	pub structured_field_priority: u32,
	pub recursive_priority: u32,
}
