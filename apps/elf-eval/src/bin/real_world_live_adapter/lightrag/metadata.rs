use super::super::*;

pub(super) fn lightrag_api_base(args: &LightragArgs) -> String {
	args.api_base.trim_end_matches('/').to_string()
}

pub(super) fn lightrag_metadata(args: &LightragArgs, run_slug: &str) -> serde_json::Value {
	serde_json::json!({
		"schema": "elf.lightrag_context_export_metadata/v1",
		"run_slug": run_slug,
		"api_base": lightrag_api_base(args),
		"query": {
			"mode": args.query_mode,
			"only_need_context": true,
			"include_references": true,
			"include_chunk_content": true,
			"enable_rerank": false,
			"top_k": args.top_k,
			"chunk_top_k": args.chunk_top_k
		},
		"docker_boundary": {
			"compose_file": "docker-compose.baseline.yml",
			"service_profile": "lightrag",
			"service": "lightrag",
			"mock_provider_service": "lightrag-mock-provider",
			"host_global_installs_required": false,
			"workspace": "/app/data/rag_storage",
			"input_dir": "/app/data/inputs",
			"data_volumes": [
				"elf-live-baseline-lightrag-rag-storage",
				"elf-live-baseline-lightrag-inputs",
				"elf-live-baseline-lightrag-prompts"
			]
		},
		"provider_boundaries": {
			"llm_binding": "openai-compatible",
			"embedding_binding": "openai-compatible",
			"embedding_dim": 64,
			"rerank_binding": "cohere-compatible",
			"rerank_enabled_for_query": false,
			"api_key_provided": args.api_key.as_deref().is_some_and(|key| !key.is_empty()),
			"operator_owned_provider_credentials_used": false
		},
		"cache_and_resource_envelope": {
			"cargo_cache": "/usr/local/cargo",
			"pip_cache": "/root/.cache/pip",
			"huggingface_cache": "/root/.cache/huggingface",
			"lightrag_storage": "/app/data/rag_storage",
			"startup_attempts": args.startup_attempts,
			"startup_interval_seconds": args.startup_interval_seconds,
			"index_attempts": args.index_attempts,
			"index_interval_seconds": args.index_interval_seconds
		},
		"source_mapping": {
			"corpus_file_source_template": "elf-real-world/{run_slug}/{job_slug}/{evidence_id}.md",
			"mapping_inputs": ["references.file_path", "references.content", "response"],
			"quality_claim": "none"
		}
	})
}
