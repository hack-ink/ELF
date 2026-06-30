use crate::{
	AGENT_ID, ElfService, Instant, LoadedJob, PayloadLevel, Result, SearchRequest, SearchResponse,
	TENANT_ID, eyre,
};

pub(super) async fn search_elf_job(
	service: &ElfService,
	loaded: &LoadedJob,
	project_id: &str,
) -> Result<(SearchResponse, f64)> {
	let started_at = Instant::now();
	let response = service
		.search_raw(SearchRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: project_id.to_string(),
			agent_id: AGENT_ID.to_string(),
			token_id: None,
			payload_level: PayloadLevel::L2,
			read_profile: "private_only".to_string(),
			query: loaded.job.prompt.content.clone(),
			top_k: Some(5),
			candidate_k: Some(20),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.map_err(|err| eyre::eyre!("ELF search_raw failed for {}: {err}", loaded.job.job_id))?;

	Ok((response, started_at.elapsed().as_secs_f64() * 1_000.0))
}
