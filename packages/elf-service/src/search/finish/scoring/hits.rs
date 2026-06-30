use crate::search::{self, ElfService, OffsetDateTime, Result, ScoredChunk};

impl ElfService {
	pub(in crate::search) async fn record_hits_if_enabled(
		&self,
		enabled: bool,
		query: &str,
		selected_results: &[ScoredChunk],
		now: OffsetDateTime,
	) -> Result<()> {
		if !enabled || selected_results.is_empty() {
			return Ok(());
		}

		let mut tx = self.db.pool.begin().await?;

		search::record_hits(&mut *tx, query, selected_results, now).await?;

		tx.commit().await?;

		Ok(())
	}
}
