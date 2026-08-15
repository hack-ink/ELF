use crate::{LightragArgs, Value, serde_json};

pub(super) fn lightrag_api_base(args: &LightragArgs) -> String {
	args.api_base.trim_end_matches('/').to_string()
}

pub(super) fn lightrag_metadata(args: &LightragArgs) -> Value {
	serde_json::json!({
		"schema": "elf.lightrag_context_export_metadata/v1",
		"index_reused": args.reuse_index,
		"index_reset_before_cold": args.reset_index,
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
			"compose_file": "docker/benchmark/compose.yml",
			"service": "lightrag",
			"host_global_installs_required": false,
			"compose_project_scoped": true,
			"workspace": "/app/data/rag_storage",
			"input_dir": "/app/data/inputs",
			"data_volumes": [
				"lightrag-data",
				"lightrag-inputs"
			]
		},
		"provider_boundaries": {
			"llm_binding": "openai-compatible",
			"embedding_binding": "openai-compatible",
			"embedding_dim": 4_096,
			"rerank_enabled_for_query": false,
			"api_key_provided": args.api_key.as_deref().is_some_and(|key| !key.is_empty()),
			"operator_owned_provider_credentials_used": true
		},
		"cache_and_resource_envelope": {
			"cargo_cache": "/usr/local/cargo",
			"clear_attempts": args.clear_attempts,
			"pip_cache": "/root/.cache/pip",
			"huggingface_cache": "/root/.cache/huggingface",
			"lightrag_storage": "/app/data/rag_storage",
			"startup_attempts": args.startup_attempts,
			"startup_interval_seconds": args.startup_interval_seconds,
			"index_attempts": args.index_attempts,
			"index_interval_seconds": args.index_interval_seconds
		},
		"source_mapping": {
			"corpus_file_source_template": "elf-real-world/{run_slug}/{job_slug}/source-{opaque_digest}.md",
			"mapping_inputs": ["references.file_path", "references.reference_id"],
			"content_inference_allowed": false
		}
	})
}
