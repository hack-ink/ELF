use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
};

use elf_config::EmbeddingProviderConfig;
use elf_service::{BoxFuture, EmbeddingProvider, Result};

pub(in crate::acceptance::graph_ingestion::tests_helpers) struct HashEmbedding {
	pub(in crate::acceptance::graph_ingestion::tests_helpers) vector_dim: u32,
}
impl EmbeddingProvider for HashEmbedding {
	fn embed<'a>(
		&'a self,
		_: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, Result<Vec<Vec<f32>>>> {
		let vector_dim = self.vector_dim as usize;
		let vectors = texts
			.iter()
			.map(|text| {
				let mut values = Vec::with_capacity(vector_dim);

				for idx in 0..vector_dim {
					let mut hasher = DefaultHasher::new();

					text.hash(&mut hasher);
					idx.hash(&mut hasher);

					let raw = hasher.finish();
					let normalized = ((raw % 2_000_000) as f32 / 1_000_000.0) - 1.0;

					values.push(normalized);
				}

				values
			})
			.collect();

		Box::pin(async move { Ok(vectors) })
	}
}
