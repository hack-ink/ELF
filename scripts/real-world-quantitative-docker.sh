#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ELF_REAL_WORLD_QUANTITATIVE_REPORT_DIR:-${ROOT_DIR}/tmp/real-world-memory/quantitative-docker}"
LIVE_QRELS_DIR="${ELF_REAL_WORLD_QUANTITATIVE_LIVE_EXPLICIT_QRELS_DIR:-${ROOT_DIR}/tmp/real-world-memory/live-explicit-qrels}"
LIVE_ADAPTER_DIR="${LIVE_QRELS_DIR}/live-adapters"
SYNC_LOG="${REPORT_DIR}/synced-artifacts.tsv"
QMD_PRODUCT_MANIFEST="${REPORT_DIR}/qmd-quantitative-product-manifest.json"
FRESHNESS_MANIFEST="${REPORT_DIR}/quantitative-artifact-freshness-manifest.json"
RUN_LIVE_EXPLICIT_QRELS="${ELF_REAL_WORLD_QUANTITATIVE_RUN_LIVE_EXPLICIT_QRELS:-1}"
RUN_LANGGRAPH="${ELF_REAL_WORLD_QUANTITATIVE_RUN_LANGGRAPH:-0}"
QMD_DIR="${ELF_REAL_WORLD_QMD_DIR:-/bench/repos/qmd}"

if [[ ! -f "/.dockerenv" ]]; then
	echo "Refusing to run the quantitative benchmark aggregate outside Docker." >&2
	echo "Use cargo make real-world-memory-quantitative-docker." >&2
	exit 1
fi

for cmd in bash cargo git jq python3; do
	if ! command -v "${cmd}" >/dev/null 2>&1; then
		echo "Missing ${cmd} in quantitative Docker benchmark runner." >&2
		exit 1
	fi
done

require_runner_image_digest() {
	local digest="${ELF_BASELINE_RUNNER_IMAGE_DIGEST:-}"
	if [[ -z "${digest}" ]]; then
		digest="${ELF_REAL_WORLD_QUANTITATIVE_RUNNER_IMAGE_DIGEST:-}"
	fi
	if [[ -z "${digest}" ]]; then
		echo "Missing baseline-runner image digest before quantitative aggregate work starts." >&2
		echo "Use cargo make real-world-memory-quantitative-docker so scripts/real-world-docker.sh can pass ELF_BASELINE_RUNNER_IMAGE_DIGEST." >&2
		exit 1
	fi
	if [[ ! "${digest}" =~ ^sha256:[0-9a-fA-F]{64}$ ]]; then
		echo "Invalid baseline-runner image digest: ${digest}" >&2
		exit 1
	fi
	export ELF_BASELINE_RUNNER_IMAGE_DIGEST="${digest}"
	export ELF_REAL_WORLD_QUANTITATIVE_RUNNER_IMAGE_DIGEST="${digest}"
}

annotate_product_manifest_from_git() {
	local manifest="$1"
	local repo_dir="$2"
	local source="$3"

	if [[ ! -d "${repo_dir}/.git" ]]; then
		return 0
	fi

	local commit
	commit="$(git -C "${repo_dir}" rev-parse HEAD)"
	jq \
		--arg commit "${commit}" \
		--arg source "${source}" \
		'.product_commit = $commit
		| .product_commit_source = $source
		| .rows |= map(.product_commit = $commit | .product_commit_source = $source)' \
		"${manifest}" >"${manifest}.tmp"
	mv "${manifest}.tmp" "${manifest}"
}

require_runner_image_digest

cd "${ROOT_DIR}"
rm -rf "${REPORT_DIR}"
mkdir -p "${REPORT_DIR}"
: >"${SYNC_LOG}"

if [[ "${RUN_LIVE_EXPLICIT_QRELS}" == "1" ]]; then
	ELF_REAL_WORLD_LIVE_EXPLICIT_QRELS_REPORT_DIR="${LIVE_QRELS_DIR}" \
		bash scripts/real-world-live-explicit-qrels.sh
fi

cargo run -p elf-eval --bin real_world_job_benchmark -- export-quantitative-product-manifest \
	--report "${LIVE_ADAPTER_DIR}/qmd-report.json" \
	--out "${QMD_PRODUCT_MANIFEST}" \
	--product qmd \
	--adapter-id qmd_live_real_world \
	--adapter-name "qmd live real-world CLI adapter"

annotate_product_manifest_from_git "${QMD_PRODUCT_MANIFEST}" "${QMD_DIR}" "git.rev_parse_head:qmd_dir"

printf 'combined-input\tqmd-live-explicit-qrels\t%s\t%s\n' \
	"${QMD_PRODUCT_MANIFEST}" \
	"current_docker_run" >>"${SYNC_LOG}"

python3 scripts/materialize-quantitative-artifact-freshness.py \
	--sync-log "${SYNC_LOG}" \
	--combined-product-manifest "${QMD_PRODUCT_MANIFEST}" \
	--out "${FRESHNESS_MANIFEST}" \
	--run-live-explicit-qrels "${RUN_LIVE_EXPLICIT_QRELS}" \
	--run-langgraph "${RUN_LANGGRAPH}"

echo "Quantitative Docker benchmark artifacts:"
echo "  ${QMD_PRODUCT_MANIFEST}"
echo "  ${FRESHNESS_MANIFEST}"
