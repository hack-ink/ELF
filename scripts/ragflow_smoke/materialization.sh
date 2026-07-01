# RAGFlow Docker evidence smoke helper functions.
# Sourced by scripts/ragflow-docker-evidence-smoke.sh.

write_artifact() {
	local generated_at out_rel manifest_rel fixture_rel report_json_rel report_md_rel docker_status git_status curl_status jq_status
	generated_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
	out_rel="$(relative_path "${OUT}")"
	manifest_rel="$(relative_path "${MANIFEST_OUT}")"
	fixture_rel="$(relative_path "${FIXTURE_PATH}")"
	report_json_rel="$(relative_path "${REPORT_JSON}")"
	report_md_rel="$(relative_path "${REPORT_MD}")"
	docker_status="$(optional_command_status docker)"
	git_status="$(optional_command_status git)"
	curl_status="$(optional_command_status curl)"
	jq_status="$(optional_command_status jq)"

	jq -n \
		--arg schema "elf.ragflow_docker_evidence_smoke/v1" \
		--arg run_id "${RUN_ID}" \
		--arg generated_at "${generated_at}" \
		--arg adapter_id "ragflow_docker_evidence_smoke" \
		--arg evidence_class "${EVIDENCE_CLASS}" \
		--arg overall_status "$(json_status "${OVERALL_STATUS}")" \
		--arg setup_status "$(json_status "${SETUP_STATUS}")" \
		--arg run_status "$(json_status "${RUN_STATUS}")" \
		--arg result_status "$(json_status "${RESULT_STATUS}")" \
		--arg failure_class "${FAILURE_CLASS}" \
		--arg failure_reason "${FAILURE_REASON}" \
		--arg out_rel "${out_rel}" \
		--arg manifest_rel "${manifest_rel}" \
		--arg fixture_rel "${fixture_rel}" \
		--arg report_json_rel "${report_json_rel}" \
		--arg report_md_rel "${report_md_rel}" \
		--arg artifact_dir "$(relative_path "${ARTIFACT_DIR}")" \
		--arg work_dir "$(relative_path "${WORK_DIR}")" \
		--arg repo_url "${RAGFLOW_REPO_URL}" \
		--arg ragflow_ref "${RAGFLOW_REF}" \
		--arg ragflow_image "${RAGFLOW_IMAGE}" \
		--arg compose_project "${COMPOSE_PROJECT}" \
		--arg cpu_gpu_mode "${CPU_GPU_MODE}" \
		--arg start_enabled "${START_RAGFLOW}" \
		--arg accept_resource_envelope "${ACCEPT_RESOURCE_ENVELOPE}" \
		--arg allow_arm "${ALLOW_ARM}" \
		--arg pull_image "${PULL_IMAGE}" \
		--arg cleanup "${CLEANUP}" \
		--arg api_base "${API_BASE}" \
		--arg api_key_provided "$([[ -n "${API_KEY}" ]] && printf true || printf false)" \
		--arg startup_time_ms "${STARTUP_TIME_MS}" \
		--arg started "${STARTED}" \
		--arg startup_attempt_count "${STARTUP_ATTEMPTS}" \
		--arg startup_interval_seconds "${STARTUP_INTERVAL_SECONDS}" \
		--arg compose_timeout_seconds "${COMPOSE_TIMEOUT_SECONDS}" \
		--arg evidence_id "${EVIDENCE_ID}" \
		--arg document_name "${DOCUMENT_NAME}" \
		--arg evidence_token "${EVIDENCE_TOKEN}" \
		--arg corpus_text "${CORPUS_TEXT}" \
		--arg dataset_id "${DATASET_ID}" \
		--arg document_id "${DOCUMENT_ID}" \
		--arg chunk_id "${CHUNK_ID}" \
		--arg vm_max_map_count "${VM_MAX_MAP_COUNT}" \
		--arg vm_max_map_count_status "${VM_MAX_MAP_COUNT_STATUS}" \
		--arg vm_max_map_count_action "${VM_MAX_MAP_COUNT_ACTION}" \
		--arg image_present "${IMAGE_PRESENT}" \
		--arg image_size_bytes "${IMAGE_SIZE_BYTES}" \
		--arg host_global_installs_required "${HOST_GLOBAL_INSTALLS_REQUIRED}" \
		--arg docker_status "${docker_status}" \
		--arg git_status "${git_status}" \
		--arg curl_status "${curl_status}" \
		--arg jq_status "${jq_status}" \
		--arg dataset_step_status "$(json_status "${DATASET_STEP_STATUS}")" \
		--arg document_step_status "$(json_status "${DOCUMENT_STEP_STATUS}")" \
		--arg chunk_step_status "$(json_status "${CHUNK_STEP_STATUS}")" \
		--arg retrieval_step_status "$(json_status "${RETRIEVAL_STEP_STATUS}")" \
		--slurpfile docker_info "${DOCKER_INFO}" \
		--slurpfile image_inspect "${IMAGE_INSPECT}" \
		--slurpfile reference_mapping "${REFERENCE_MAPPING}" \
		--rawfile docker_df "${DOCKER_DF}" \
		--rawfile compose_up_log "${COMPOSE_UP_LOG}" \
		--rawfile compose_down_log "${COMPOSE_DOWN_LOG}" \
		--slurpfile dataset_response "${DATASET_RESPONSE}" \
		--slurpfile document_response "${DOCUMENT_RESPONSE}" \
		--slurpfile chunk_response "${CHUNK_RESPONSE}" \
		--slurpfile retrieval_response "${RETRIEVAL_RESPONSE}" \
		--slurpfile scored_benchmark "${SCORED_BENCHMARK}" \
		--slurpfile startup_attempts <(jq -s '.' "${STARTUP_ATTEMPTS_JSONL}") \
		'{
			schema: $schema,
			run_id: $run_id,
			generated_at: $generated_at,
			adapter_id: $adapter_id,
			evidence_class: $evidence_class,
			overall_status: $overall_status,
			status_source: "smoke_materialization",
			scored_benchmark: $scored_benchmark[0],
			no_quality_claim: true,
			failure: (
				if $failure_class == "" then null
				else {
					class: $failure_class,
					reason: $failure_reason
				}
				end
			),
			artifacts: {
				smoke: $out_rel,
				external_adapter_manifest: $manifest_rel,
				generated_fixture: $fixture_rel,
				scored_report_json: $report_json_rel,
				scored_report_markdown: $report_md_rel,
				artifact_dir: $artifact_dir,
				work_dir: $work_dir
			},
			upstream: {
				repository: $repo_url,
				ref: $ragflow_ref,
				quickstart: "https://ragflow.io/docs/",
				http_api_reference: "https://raw.githubusercontent.com/infiniflow/ragflow/main/docs/references/http_api_reference.md",
				api_key_guide: "https://ragflow.io/docs/acquire_ragflow_api_key"
			},
			docker_boundary: {
				status: $setup_status,
				official_compose_path: "ragflow/docker/docker-compose.yml",
				compose_project: $compose_project,
				image: $ragflow_image,
				device: $cpu_gpu_mode,
				start_enabled: ($start_enabled == "1"),
				resource_envelope_accepted: ($accept_resource_envelope == "1"),
				allow_arm: ($allow_arm == "1"),
				pull_image_requested: ($pull_image == "1"),
				cleanup_requested: ($cleanup == "1"),
				host_global_installs_required: ($host_global_installs_required == "true"),
				tooling: {
					docker: $docker_status,
					git: $git_status,
					curl: $curl_status,
					jq: $jq_status
				}
			},
			setup: {
				status: $setup_status,
				command: "cargo make smoke-ragflow-docker",
				live_command: "ELF_RAGFLOW_SMOKE_START=1 ELF_RAGFLOW_SMOKE_ACCEPT_RESOURCE_ENVELOPE=1 cargo make smoke-ragflow-docker",
				started: ($started == "true"),
				startup_time_ms: (if $startup_time_ms == "" then null else ($startup_time_ms | tonumber) end),
				vm_max_map_count: {
					status: $vm_max_map_count_status,
					observed: (if $vm_max_map_count == "" then null else $vm_max_map_count end),
					required_min: 262144,
					action: $vm_max_map_count_action
				},
				image: {
					present: ($image_present == "true"),
					size_bytes: (if $image_size_bytes == "" then null else ($image_size_bytes | tonumber) end),
					official_compressed_size_note: "RAGFlow quickstart lists the stable image at about 2 GB compressed.",
					official_expanded_size_note: "RAGFlow quickstart says the image expands to about 7 GB once unpacked.",
					inspect: ($image_inspect[0] // [])
				},
				resource_envelope: {
					official_min_cpu_cores: 4,
					official_min_ram_gb: 16,
					official_min_disk_gb: 50,
					docker_info: ($docker_info[0] // {}),
					docker_system_df: $docker_df
				},
				provider_boundaries: {
					ragflow_api_base: $api_base,
					ragflow_api_key_provided: ($api_key_provided == "true"),
					operator_owned_provider_credentials_used: false,
					private_corpus_used: false,
					generated_public_corpus_only: true,
					external_llm_quality_scoring_claimed: false
				},
				retry_behavior: {
					startup_poll_attempts_configured: ($startup_attempt_count | tonumber),
					startup_interval_seconds: ($startup_interval_seconds | tonumber),
					compose_timeout_seconds: ($compose_timeout_seconds | tonumber),
					startup_attempts: ($startup_attempts[0] // [])
				},
				log_excerpt: {
					compose_up: ($compose_up_log | split("\n") | .[0:40]),
					compose_down: ($compose_down_log | split("\n") | .[0:20])
				}
			},
			corpus: {
				profile: "generated_public",
				evidence_id: $evidence_id,
				document_name: $document_name,
				evidence_token: $evidence_token,
				text: $corpus_text,
				dataset_id: (if $dataset_id == "" then null else $dataset_id end),
				document_id: (if $document_id == "" then null else $document_id end),
				chunk_id: (if $chunk_id == "" then null else $chunk_id end)
			},
			run: {
				status: $run_status,
				steps: {
					dataset_creation: {
						status: $dataset_step_status,
						request_artifact: "dataset-create-request.json",
						response_artifact: "dataset-create-response.json",
						response: ($dataset_response[0] // null)
					},
					document_creation: {
						status: $document_step_status,
						request_artifact: "document-create-request.json",
						response_artifact: "document-create-response.json",
						response: ($document_response[0] // null)
					},
					chunk_ingest: {
						status: $chunk_step_status,
						request_artifact: "chunk-create-request.json",
						response_artifact: "chunk-create-response.json",
						response: ($chunk_response[0] // null)
					},
					retrieval_query: {
						status: $retrieval_step_status,
						request_artifact: "retrieval-request.json",
						response_artifact: "retrieval-response.json",
						response: ($retrieval_response[0] // null)
					}
				}
			},
			result: {
				status: $result_status,
				evidence: "RAGFlow retrieval reference chunks are mapped to real_world_job evidence ids when content or document metadata matches the generated public corpus.",
				reference_chunk_count: (($reference_mapping[0] // []) | length),
				mapped_reference_chunk_count: (($reference_mapping[0] // []) | map(select((.evidence_ids // []) | length > 0)) | length)
			},
			evidence_mapping: {
				expected_evidence_ids: [$evidence_id],
				reference_chunks: ($reference_mapping[0] // []),
				field_mapping: {
					"id": "chunk_id",
					"document_id": "document_id",
					"document_name_or_document_keyword": "document_name",
					"dataset_id_or_kb_id": "dataset_id",
					"content_or_content_with_weight": "content",
					"positions": "positions",
					"similarity": "similarity",
					"vector_similarity": "vector_similarity",
					"term_similarity": "term_similarity"
				}
			}
		}' >"${OUT}"
}
