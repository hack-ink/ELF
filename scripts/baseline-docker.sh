#!/usr/bin/env bash
set -euo pipefail

profile="${1:-}"
if [ -z "$profile" ]; then
	echo "usage: scripts/baseline-docker.sh <profile>" >&2
	exit 2
fi

head="$(git rev-parse HEAD)"
if [ -n "$(git status --porcelain)" ]; then
	head="$head+dirty"
fi

run_baseline() {
	docker compose -f docker-compose.baseline.yml run --build --rm baseline-runner
}

selected_projects_or_default() {
	local selected_projects
	selected_projects="$(printenv ELF_BASELINE_PROJECTS || true)"
	if [ -z "$selected_projects" ]; then
		selected_projects="ELF"
	fi
	printf '%s' "$selected_projects"
}

case "$profile" in
live)
	export ELF_BASELINE_ELF_HEAD="$head"
	run_baseline
	;;
backfill)
	selected_projects="$(selected_projects_or_default)"
	selected_profile="$(printenv ELF_BASELINE_PROFILE || true)"
	if [ -z "$selected_profile" ]; then
		selected_profile="backfill"
	fi
	backfill_docs="$(printenv ELF_BASELINE_BACKFILL_DOCS || true)"
	if [ -z "$backfill_docs" ]; then
		backfill_docs="2000"
	fi
	elf_timeout="$(printenv ELF_BASELINE_ELF_TIMEOUT_SECONDS || true)"
	if [ -z "$elf_timeout" ]; then
		elf_timeout="3600"
	fi
	max_elf_seconds="$(printenv ELF_BASELINE_MAX_ELF_SECONDS || true)"
	if [ -z "$max_elf_seconds" ]; then
		max_elf_seconds="3600"
	fi
	export ELF_BASELINE_ELF_HEAD="$head"
	export ELF_BASELINE_PROJECTS="$selected_projects"
	export ELF_BASELINE_PROFILE="$selected_profile"
	export ELF_BASELINE_BACKFILL_DOCS="$backfill_docs"
	export ELF_BASELINE_ELF_TIMEOUT_SECONDS="$elf_timeout"
	export ELF_BASELINE_MAX_ELF_SECONDS="$max_elf_seconds"
	run_baseline
	;;
openmemory-ui-export-readback)
	export ELF_BASELINE_ELF_HEAD="$head"
	export ELF_BASELINE_PROJECTS=mem0
	run_baseline
	;;
production-synthetic)
	selected_projects="$(selected_projects_or_default)"
	export ELF_BASELINE_ELF_HEAD="$head"
	export ELF_BASELINE_PROJECTS="$selected_projects"
	export ELF_BASELINE_PROFILE=production-synthetic
	run_baseline
	;;
production-private)
	manifest="$(printenv ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST || true)"
	if [ -z "$manifest" ]; then
		echo "ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST is required for baseline-production-private" >&2
		exit 1
	fi
	selected_projects="$(selected_projects_or_default)"
	export ELF_BASELINE_ELF_HEAD="$head"
	export ELF_BASELINE_PROJECTS="$selected_projects"
	export ELF_BASELINE_PROFILE=production-private
	run_baseline
	;;
production-private-addendum)
	manifest="$(printenv ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST || true)"
	if [ -z "$manifest" ]; then
		echo "ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST is required for baseline-production-private-addendum" >&2
		exit 1
	fi
	selected_projects="$(selected_projects_or_default)"
	addendum="$(printenv ELF_BASELINE_PRIVATE_ADDENDUM || true)"
	if [ -z "$addendum" ]; then
		addendum="tmp/live-baseline/private-production-addendum.md"
	fi
	export ELF_BASELINE_ELF_HEAD="$head"
	export ELF_BASELINE_PROJECTS="$selected_projects"
	export ELF_BASELINE_PROFILE=production-private
	run_baseline
	ELF_BASELINE_MARKDOWN_REPORT="$addendum" bash scripts/live-baseline-report-to-md.sh
	echo "Private production addendum: $addendum"
	;;
backfill-10k)
	backfill_docs="$(printenv ELF_BASELINE_BACKFILL_DOCS || true)"
	if [ -z "$backfill_docs" ]; then
		backfill_docs="10000"
	fi
	elf_timeout="$(printenv ELF_BASELINE_ELF_TIMEOUT_SECONDS || true)"
	if [ -z "$elf_timeout" ]; then
		elf_timeout="14400"
	fi
	max_elf_seconds="$(printenv ELF_BASELINE_MAX_ELF_SECONDS || true)"
	if [ -z "$max_elf_seconds" ]; then
		max_elf_seconds="$elf_timeout"
	fi
	export ELF_BASELINE_ELF_HEAD="$head"
	export ELF_BASELINE_PROJECTS=ELF
	export ELF_BASELINE_PROFILE=backfill
	export ELF_BASELINE_BACKFILL_DOCS="$backfill_docs"
	export ELF_BASELINE_ELF_TIMEOUT_SECONDS="$elf_timeout"
	export ELF_BASELINE_MAX_ELF_SECONDS="$max_elf_seconds"
	run_baseline
	;;
backfill-100k)
	enabled="$(printenv ELF_BASELINE_ENABLE_EXPENSIVE || true)"
	if [ "$enabled" != "1" ]; then
		echo "ELF_BASELINE_ENABLE_EXPENSIVE=1 is required for baseline-backfill-100k-docker" >&2
		exit 1
	fi
	backfill_docs="$(printenv ELF_BASELINE_BACKFILL_DOCS || true)"
	if [ -z "$backfill_docs" ]; then
		backfill_docs="100000"
	fi
	elf_timeout="$(printenv ELF_BASELINE_ELF_TIMEOUT_SECONDS || true)"
	if [ -z "$elf_timeout" ]; then
		elf_timeout="86400"
	fi
	max_elf_seconds="$(printenv ELF_BASELINE_MAX_ELF_SECONDS || true)"
	if [ -z "$max_elf_seconds" ]; then
		max_elf_seconds="$elf_timeout"
	fi
	export ELF_BASELINE_ELF_HEAD="$head"
	export ELF_BASELINE_PROJECTS=ELF
	export ELF_BASELINE_PROFILE=backfill
	export ELF_BASELINE_BACKFILL_DOCS="$backfill_docs"
	export ELF_BASELINE_ELF_TIMEOUT_SECONDS="$elf_timeout"
	export ELF_BASELINE_MAX_ELF_SECONDS="$max_elf_seconds"
	run_baseline
	;;
soak)
	soak_seconds="$(printenv ELF_BASELINE_SOAK_SECONDS || true)"
	if [ -z "$soak_seconds" ]; then
		soak_seconds="3600"
	fi
	elf_timeout="$(printenv ELF_BASELINE_ELF_TIMEOUT_SECONDS || true)"
	if [ -z "$elf_timeout" ]; then
		elf_timeout="$((soak_seconds + 1800))"
	fi
	max_elf_seconds="$(printenv ELF_BASELINE_MAX_ELF_SECONDS || true)"
	if [ -z "$max_elf_seconds" ]; then
		max_elf_seconds="$elf_timeout"
	fi
	export ELF_BASELINE_ELF_HEAD="$head"
	export ELF_BASELINE_PROJECTS=ELF
	export ELF_BASELINE_PROFILE=stress
	export ELF_BASELINE_SOAK_SECONDS="$soak_seconds"
	export ELF_BASELINE_ELF_TIMEOUT_SECONDS="$elf_timeout"
	export ELF_BASELINE_MAX_ELF_SECONDS="$max_elf_seconds"
	run_baseline
	;;
*)
	echo "unknown baseline profile: $profile" >&2
	exit 2
	;;
esac
