use crate::search::{ElfService, FinishSearchPolicies, RankingRequestOverride, Result, ranking};

impl ElfService {
	pub(in crate::search) fn resolve_finish_search_policies(
		&self,
		ranking_override: Option<&RankingRequestOverride>,
	) -> Result<FinishSearchPolicies> {
		let blend_policy = ranking::resolve_blend_policy(
			&self.cfg.ranking.blend,
			ranking_override.and_then(|override_| override_.blend.as_ref()),
		)?;
		let diversity_policy = ranking::resolve_diversity_policy(
			&self.cfg.ranking.diversity,
			ranking_override.and_then(|override_| override_.diversity.as_ref()),
		)?;
		let retrieval_sources_policy = ranking::resolve_retrieval_sources_policy(
			&self.cfg.ranking.retrieval_sources,
			ranking_override.and_then(|override_| override_.retrieval_sources.as_ref()),
		)?;
		let policy_snapshot = ranking::build_policy_snapshot(
			&self.cfg,
			&blend_policy,
			&diversity_policy,
			&retrieval_sources_policy,
			ranking_override,
		);
		let policy_hash = ranking::hash_policy_snapshot(&policy_snapshot)?;
		let policy_id = format!("ranking_v2:{}", &policy_hash[..12.min(policy_hash.len())]);

		Ok(FinishSearchPolicies {
			blend_policy,
			diversity_policy,
			retrieval_sources_policy,
			policy_snapshot,
			policy_id,
		})
	}
}
