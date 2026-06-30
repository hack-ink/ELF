use crate::{
	Error,
	search::{
		self, ElfService, ExpansionMode, MAX_CANDIDATE_K, RawSearchExecutionContext, RawSearchPath,
		Result, SearchFilter, SearchRequest, Uuid, ranking,
	},
};

impl ElfService {
	pub(in crate::search) fn prepare_raw_search_execution(
		&self,
		req: SearchRequest,
		path: RawSearchPath,
	) -> Result<RawSearchExecutionContext> {
		let tenant_id = req.tenant_id.trim().to_string();
		let project_id = req.project_id.trim().to_string();
		let agent_id = req.agent_id.trim().to_string();
		let token_id = req
			.token_id
			.as_deref()
			.map(str::trim)
			.filter(|value| !value.is_empty())
			.map(|value| value.to_string());

		search::validate_search_request_inputs(
			tenant_id.as_str(),
			project_id.as_str(),
			agent_id.as_str(),
			req.query.as_str(),
		)?;

		let top_k = req.top_k.unwrap_or(self.cfg.memory.top_k).max(1);
		let candidate_k = req.candidate_k.unwrap_or(self.cfg.memory.candidate_k).max(top_k);
		let requested_candidate_k = candidate_k;
		let filter = req
			.filter
			.as_ref()
			.map(SearchFilter::parse)
			.transpose()
			.map_err(|err| Error::InvalidRequest { message: err.to_string() })?;
		let effective_candidate_k = if filter.is_some() {
			requested_candidate_k.saturating_mul(3).min(MAX_CANDIDATE_K).max(top_k)
		} else {
			requested_candidate_k
		};
		let query = req.query;
		let read_profile = req.read_profile;
		let record_hits_enabled = req.record_hits.unwrap_or(false);
		let ranking_override = req.ranking;
		let retrieval_sources_policy = ranking::resolve_retrieval_sources_policy(
			&self.cfg.ranking.retrieval_sources,
			ranking_override.as_ref().and_then(|override_| override_.retrieval_sources.as_ref()),
		)?;
		let expansion_mode = match path {
			RawSearchPath::Quick => ExpansionMode::Off,
			RawSearchPath::Planned => ranking::resolve_expansion_mode(&self.cfg),
		};
		let trace_id = Uuid::new_v4();
		let project_context_description = self
			.resolve_project_context_description(tenant_id.as_str(), project_id.as_str())
			.map(|value| value.to_string());
		let allowed_scopes = ranking::resolve_scopes(&self.cfg, read_profile.as_str())?;
		let policies = self.resolve_finish_search_policies(ranking_override.as_ref())?;

		Ok(RawSearchExecutionContext {
			tenant_id,
			project_id,
			agent_id,
			token_id,
			top_k,
			candidate_k,
			requested_candidate_k,
			effective_candidate_k,
			filter,
			query,
			read_profile,
			payload_level: req.payload_level,
			record_hits_enabled,
			ranking_override,
			retrieval_sources_policy,
			expansion_mode,
			trace_id,
			project_context_description,
			allowed_scopes,
			policies,
		})
	}
}
