use crate::{
	BaselineRuntime, ConsolidationMaterializationEvidence, DreamingReadbackOutput, ElfService,
	IngestedCorpus, KnowledgeMaterializationEvidence, LoadedJob, Result, Uuid, Value,
};

pub(super) struct OptionalElfMaterializations {
	pub(super) pages: Vec<Value>,
	pub(super) knowledge: Option<KnowledgeMaterializationEvidence>,
	pub(super) consolidation_response: Option<Value>,
	pub(super) consolidation: Option<ConsolidationMaterializationEvidence>,
	pub(super) dreaming_readback: Option<DreamingReadbackOutput>,
	pub(super) failure: Option<String>,
}

pub(super) async fn materialize_optional_elf_surfaces(
	runtime: &BaselineRuntime,
	service: &ElfService,
	loaded: &LoadedJob,
	ingested: &IngestedCorpus,
	project_id: &str,
	trace_id: Uuid,
	adapter_id: &str,
) -> Result<OptionalElfMaterializations> {
	let (pages, knowledge, knowledge_failure) =
		match crate::materialize_elf_knowledge(service, loaded, ingested, adapter_id).await {
			Ok(output) => output,
			Err(err) if loaded.job.suite == "knowledge_compilation" =>
				(Vec::new(), None, Some(format!("live_adapter.knowledge: {err}"))),
			Err(_) => (Vec::new(), None, None),
		};
	let (consolidation_response, consolidation, consolidation_failure) =
		match crate::materialize_elf_consolidation(runtime, service, loaded, ingested, adapter_id)
			.await
		{
			Ok(output) => output,
			Err(err) if loaded.job.suite == "consolidation" =>
				(None, None, Some(format!("live_adapter.consolidation: {err}"))),
			Err(_) => (None, None, None),
		};
	let dreaming_readback =
		crate::materialize_elf_dreaming_readback(service, loaded, project_id, trace_id, adapter_id)
			.await?;
	let dreaming_failure = dreaming_readback.as_ref().and_then(|output| {
		if output.materialization.missing_source_refs.is_empty() {
			None
		} else {
			Some(format!(
				"live_adapter.dreaming_readback missing source refs: {}",
				output.materialization.missing_source_refs.join(", ")
			))
		}
	});

	Ok(OptionalElfMaterializations {
		pages,
		knowledge,
		consolidation_response,
		consolidation,
		dreaming_readback,
		failure: knowledge_failure.or(consolidation_failure).or(dreaming_failure),
	})
}
