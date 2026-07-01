# RAGFlow Docker evidence smoke helper functions.
# Sourced by scripts/ragflow-docker-evidence-smoke.sh.

run_api_smoke() {
	local dataset_name="${RUN_ID}"

	jq -n --arg name "${dataset_name}" '{
		name: $name,
		description: "Generated public ELF RAGFlow Docker evidence smoke corpus.",
		permission: "me",
		chunk_method: "manual",
		parser_config: {"raptor": {"use_raptor": false}}
	}' >"${DATASET_REQUEST}"

	if api_json_request POST "/api/v1/datasets" "${DATASET_REQUEST}" "${DATASET_RESPONSE}" \
		&& response_code_ok "${DATASET_RESPONSE}"; then
		DATASET_STEP_STATUS="pass"
		DATASET_ID="$(extract_id "${DATASET_RESPONSE}")"
	else
		DATASET_STEP_STATUS="incomplete"
		RUN_STATUS="incomplete"
		RESULT_STATUS="incomplete"
		OVERALL_STATUS="incomplete"
		FAILURE_CLASS="ragflow_dataset_create_failed"
		FAILURE_REASON="RAGFlow dataset creation did not return a successful response."
		return 0
	fi

	if [[ -z "${DATASET_ID}" ]]; then
		DATASET_STEP_STATUS="incomplete"
		RUN_STATUS="incomplete"
		RESULT_STATUS="incomplete"
		OVERALL_STATUS="incomplete"
		FAILURE_CLASS="ragflow_dataset_id_missing"
		FAILURE_REASON="RAGFlow dataset creation succeeded but no dataset id was found in the response."
		return 0
	fi

	jq -n --arg name "${DOCUMENT_NAME}" '{name: $name}' >"${DOCUMENT_REQUEST}"

	if api_json_request POST "/api/v1/datasets/${DATASET_ID}/documents?type=empty" \
		"${DOCUMENT_REQUEST}" "${DOCUMENT_RESPONSE}" \
		&& response_code_ok "${DOCUMENT_RESPONSE}"; then
		DOCUMENT_STEP_STATUS="pass"
		DOCUMENT_ID="$(extract_id "${DOCUMENT_RESPONSE}")"
	else
		DOCUMENT_STEP_STATUS="incomplete"
		RUN_STATUS="incomplete"
		RESULT_STATUS="incomplete"
		OVERALL_STATUS="incomplete"
		FAILURE_CLASS="ragflow_document_create_failed"
		FAILURE_REASON="RAGFlow empty document creation did not return a successful response."
		return 0
	fi

	if [[ -z "${DOCUMENT_ID}" ]]; then
		DOCUMENT_STEP_STATUS="incomplete"
		RUN_STATUS="incomplete"
		RESULT_STATUS="incomplete"
		OVERALL_STATUS="incomplete"
		FAILURE_CLASS="ragflow_document_id_missing"
		FAILURE_REASON="RAGFlow empty document creation succeeded but no document id was found in the response."
		return 0
	fi

	jq -n \
		--arg content "${CORPUS_TEXT}" \
		--arg token "${EVIDENCE_TOKEN}" \
		'{
			content: $content,
			important_keywords: [$token],
			questions: ["Which evidence token should map to ragflow-smoke-anchor?"]
		}' >"${CHUNK_REQUEST}"

	if api_json_request POST "/api/v1/datasets/${DATASET_ID}/documents/${DOCUMENT_ID}/chunks" \
		"${CHUNK_REQUEST}" "${CHUNK_RESPONSE}" \
		&& response_code_ok "${CHUNK_RESPONSE}"; then
		CHUNK_STEP_STATUS="pass"
		CHUNK_ID="$(extract_id "${CHUNK_RESPONSE}")"
	else
		CHUNK_STEP_STATUS="incomplete"
		RUN_STATUS="incomplete"
		RESULT_STATUS="incomplete"
		OVERALL_STATUS="incomplete"
		FAILURE_CLASS="ragflow_chunk_create_failed"
		FAILURE_REASON="RAGFlow chunk creation did not return a successful response."
		return 0
	fi

	jq -n \
		--arg question "Which RAGFlow smoke evidence token maps to ragflow-smoke-anchor?" \
		--arg dataset_id "${DATASET_ID}" \
		--arg document_id "${DOCUMENT_ID}" \
		'{
			question: $question,
			dataset_ids: [$dataset_id],
			document_ids: [$document_id],
			page: 1,
			page_size: 5,
			similarity_threshold: 0.0,
			vector_similarity_weight: 0.0,
			top_k: 5,
			keyword: true,
			highlight: false
		}' >"${RETRIEVAL_REQUEST}"

	if api_json_request POST "/api/v1/retrieval" "${RETRIEVAL_REQUEST}" "${RETRIEVAL_RESPONSE}" \
		&& response_code_ok "${RETRIEVAL_RESPONSE}"; then
		RETRIEVAL_STEP_STATUS="pass"
	else
		RETRIEVAL_STEP_STATUS="incomplete"
		RUN_STATUS="incomplete"
		RESULT_STATUS="incomplete"
		OVERALL_STATUS="incomplete"
		FAILURE_CLASS="ragflow_retrieval_failed"
		FAILURE_REASON="RAGFlow retrieval did not return a successful response."
		return 0
	fi

	jq \
		--arg evidence_id "${EVIDENCE_ID}" \
		--arg token "${EVIDENCE_TOKEN}" \
		--arg document_name "${DOCUMENT_NAME}" '
		def chunk_array:
			if (.data.chunks? | type) == "array" then .data.chunks
			elif (.reference.chunks? | type) == "array" then .reference.chunks
			else [] end;
		chunk_array
		| map({
			chunk_id: (.id // .chunk_id // ""),
			content: (.content // .content_with_weight // ""),
			document_id: (.document_id // .doc_id // ""),
			document_name: (.document_name // .document_keyword // .doc_name // .docnm_kwd // ""),
			dataset_id: (.dataset_id // .kb_id // ""),
			positions: (.positions // []),
			similarity: (.similarity // null),
			vector_similarity: (.vector_similarity // null),
			term_similarity: (.term_similarity // null),
			evidence_ids: (
				if (((.content // .content_with_weight // "") | contains($token))
					or ((.document_name // .document_keyword // .doc_name // .docnm_kwd // "") == $document_name))
				then [$evidence_id]
				else []
				end
			),
			mapping_status: (
				if ((.content // .content_with_weight // "") | contains($token)) then "matched_content"
				elif ((.document_name // .document_keyword // .doc_name // .docnm_kwd // "") == $document_name) then "matched_document"
				else "unmatched"
				end
			)
		})' "${RETRIEVAL_RESPONSE}" >"${REFERENCE_MAPPING}"

	RUN_STATUS="pass"
	EVIDENCE_CLASS="live_real_world"

	if jq -e --arg evidence_id "${EVIDENCE_ID}" '
		length > 0 and any(.[]; (.evidence_ids // []) | index($evidence_id))
	' "${REFERENCE_MAPPING}" >/dev/null; then
		RESULT_STATUS="pass"
		OVERALL_STATUS="pass"
		FAILURE_CLASS=""
		FAILURE_REASON=""
	else
		RESULT_STATUS="wrong_result"
		OVERALL_STATUS="wrong_result"
		FAILURE_CLASS="ragflow_reference_mapping_missing"
		FAILURE_REASON="RAGFlow retrieval returned chunks but none mapped to the generated evidence id."
	fi
}

cleanup_stack() {
	local repo_dir="${WORK_DIR}/ragflow"

	if [[ "${STARTED}" != "true" || "${CLEANUP}" != "1" || ! -d "${repo_dir}/docker" ]]; then
		return 0
	fi

	(
		cd "${repo_dir}/docker"
		docker compose -p "${COMPOSE_PROJECT}" -f docker-compose.yml down -v
	) >"${COMPOSE_DOWN_LOG}" 2>&1 || true
}
