# RAGFlow Docker evidence smoke helper functions.
# Sourced by scripts/ragflow-docker-evidence-smoke.sh.

write_fixture() {
	local result_status reason
	result_status="$(json_status "${RESULT_STATUS}")"
	reason="${FAILURE_REASON}"

	jq -n \
		--arg run_id "${RUN_ID}" \
		--arg evidence_id "${EVIDENCE_ID}" \
		--arg evidence_token "${EVIDENCE_TOKEN}" \
		--arg corpus_text "${CORPUS_TEXT}" \
		--arg result_status "${result_status}" \
		--arg failure_reason "${reason}" \
		'{
			schema: "elf.real_world_job/v1",
			job_id: "ragflow-evidence-smoke-001",
			suite: "retrieval",
			title: "Map RAGFlow reference chunks to generated evidence",
			corpus: {
				corpus_id: "ragflow-generated-public-smoke",
				profile: "generated_public",
				items: [
					{
						evidence_id: $evidence_id,
						kind: "document",
						text: $corpus_text,
						source_ref: {
							schema: "source_ref/v1",
							resolver: "ragflow_smoke/v1",
							ref: {
								run_id: $run_id,
								evidence_token: $evidence_token
							}
						},
						created_at: "2026-06-10T00:00:00Z"
					}
				],
				adapter_response: {
					adapter_id: "ragflow_docker_evidence_smoke",
					answer: {
						content: (
							if $result_status == "pass" then
								"RAGFlow returned reference chunks that map to the generated ragflow-smoke-anchor evidence id."
							else
								""
							end
						),
						claims: (
							if $result_status == "pass" then
								[
									{
										claim_id: "ragflow_reference_mapping",
										text: "RAGFlow reference chunks map to the generated ragflow-smoke-anchor evidence id.",
										evidence_ids: [$evidence_id],
										confidence: "derived_from_ragflow_reference_chunk_mapping"
									}
								]
							else
								[]
							end
						),
						evidence_ids: (if $result_status == "pass" then [$evidence_id] else [] end),
						latency_ms: 0.0,
						cost: {
							currency: "USD",
							amount: 0.0,
							input_tokens: 0,
							output_tokens: 0
						}
					}
				}
			},
			timeline: [
				{
					event_id: "ragflow-smoke-corpus-generated",
					ts: "2026-06-10T00:00:00Z",
					actor: "system",
					action: "generated_public_corpus",
					evidence_ids: [$evidence_id],
					summary: "The RAGFlow smoke generated a tiny public corpus for reference chunk mapping."
				}
			],
			prompt: {
				role: "user",
				content: "Which RAGFlow smoke evidence token maps to the generated reference chunk?",
				job_mode: "answer",
				constraints: ["cite_evidence", "avoid_broad_quality_claims"]
			},
			expected_answer: {
				must_include: [
					{
						claim_id: "ragflow_reference_mapping",
						text: "RAGFlow reference chunks map to the generated ragflow-smoke-anchor evidence id."
					}
				],
				must_not_include: ["RAGFlow passed a broad graph/RAG quality benchmark."],
				evidence_links: {
					ragflow_reference_mapping: [$evidence_id]
				},
				answer_type: "direct_answer",
				accepted_alternates: [],
				requires_caveat: true,
				requires_refusal: false
			},
			required_evidence: [
				{
					evidence_id: $evidence_id,
					claim_id: "ragflow_reference_mapping",
					requirement: "cite",
					quote: "ragflow-smoke-anchor evidence id"
				}
			],
			negative_traps: [],
			scoring_rubric: {
				dimensions: {
					answer_correctness: {
						weight: 0.3,
						max_points: 1.0,
						criteria: "States the generated evidence mapping without broad quality claims."
					},
					evidence_grounding: {
						weight: 0.45,
						max_points: 1.0,
						criteria: "Maps returned RAGFlow reference chunks to the generated evidence id."
					},
					trap_avoidance: {
						weight: 0.15,
						max_points: 1.0,
						criteria: "Does not claim broad RAGFlow quality from the tiny smoke."
					},
					latency_resource: {
						weight: 0.1,
						max_points: 1.0,
						criteria: "Records setup, resource, provider, and reference-mapping boundaries."
					}
				},
				pass_threshold: 0.75,
				hard_fail_rules: []
			},
			allowed_uncertainty: {
				can_answer_unknown: false,
				acceptable_phrases: ["tiny generated corpus", "reference chunk smoke only"],
				fallback_action: "state_blocker"
			},
			operator_debug: null,
			encoding: {},
			memory_evolution: null,
			tags: ["external_adapter", "generated_public", "ragflow", "no_live_claim"]
		}
		| if ["blocked", "incomplete", "not_encoded"] | index($result_status) then
			.encoding = {status: $result_status, reason: $failure_reason}
		else
			.
		end' >"${FIXTURE_PATH}"
}
