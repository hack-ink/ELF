# RAGFlow Docker evidence smoke helper functions.
# Sourced by scripts/ragflow-docker-evidence-smoke.sh.

write_summary() {
	jq -n \
		--slurpfile materialization "${OUT}" \
		--slurpfile manifest "${MANIFEST_OUT}" \
		--slurpfile report "${REPORT_JSON}" \
		'{
			schema: "elf.ragflow_docker_smoke_summary/v1",
				generated_at: (now | todateiso8601),
				adapter_id: "ragflow_docker_evidence_smoke",
				evidence_class: $materialization[0].evidence_class,
				status_boundary: {
					materialization: "setup/run/evidence-mapping state emitted by the smoke runner",
					manifest: "external adapter declaration consumed by the scorer",
					scored_benchmark: "post-score real_world_job outcome; use this for quality status"
				},
				scored_benchmark: $materialization[0].scored_benchmark,
				materialization: $materialization[0],
				manifest: {
					json: ($materialization[0].artifacts.external_adapter_manifest // "tmp/real-world-memory/ragflow-smoke/memory_projects_manifest.ragflow-smoke.json"),
					status_source: "external_adapter_manifest_pre_score",
					summary: $manifest[0].adapters[0].overall_status,
					suites: $manifest[0].adapters[0].suites
			},
			report: {
				json: ($materialization[0].artifacts.scored_report_json // "tmp/real-world-memory/ragflow-smoke/ragflow-report.json"),
				markdown: ($materialization[0].artifacts.scored_report_markdown // "tmp/real-world-memory/ragflow-smoke/ragflow-report.md"),
				summary: $report[0].summary,
				suites: $report[0].suites
			}
		}' >"${SUMMARY_OUT}"
}

write_outputs() {
	write_scored_benchmark
	write_artifact
	write_manifest
	write_fixture
	write_scored_report
	write_scored_benchmark
	write_artifact
	write_summary
	echo "RAGFlow smoke artifact: ${OUT}"
	echo "RAGFlow smoke manifest: ${MANIFEST_OUT}"
	echo "RAGFlow smoke report: ${REPORT_JSON}"
	echo "RAGFlow smoke summary: ${SUMMARY_OUT}"
}
