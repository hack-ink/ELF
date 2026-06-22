#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="${1:-run}"

CONFIG_PATH="${ELF_LOCAL_AGENT_CONFIG:-config/local/elf.docker.toml}"
HTTP_BASE="${ELF_LOCAL_AGENT_HTTP:-http://127.0.0.1:51892}"
ADMIN_BASE="${ELF_LOCAL_AGENT_ADMIN:-http://127.0.0.1:51891}"
TENANT_ID="${ELF_LOCAL_AGENT_TENANT:-local-tenant}"
PROJECT_ID="${ELF_LOCAL_AGENT_PROJECT:-local-project}"
AGENT_ID="${ELF_LOCAL_AGENT_AGENT:-local-agent}"
READ_PROFILE="${ELF_LOCAL_AGENT_READ_PROFILE:-private_plus_project}"
OUT_DIR="${ELF_LOCAL_AGENT_OUT_DIR:-tmp/local-agent-loop}"

SERVICE_PIDS=()
JSON_BIN=""

usage() {
	cat <<'USAGE'
Usage:
  scripts/local-agent-loop.sh [run|demo]

Modes:
  run   Start local Docker dependencies, build/start ELF services, and run the demo.
  demo  Run the demo against an already running local ELF API, worker, and admin API.

Environment overrides:
  ELF_LOCAL_AGENT_CONFIG       Config path. Default: config/local/elf.docker.toml
  ELF_LOCAL_AGENT_HTTP         Public HTTP base URL. Default: http://127.0.0.1:51892
  ELF_LOCAL_AGENT_ADMIN        Admin HTTP base URL. Default: http://127.0.0.1:51891
  ELF_LOCAL_AGENT_TENANT       Tenant header. Default: local-tenant
  ELF_LOCAL_AGENT_PROJECT      Project header. Default: local-project
  ELF_LOCAL_AGENT_AGENT        Agent header. Default: local-agent
  ELF_LOCAL_AGENT_READ_PROFILE Read profile header. Default: private_plus_project
  ELF_LOCAL_AGENT_OUT_DIR      Artifact directory. Default: tmp/local-agent-loop
USAGE
}

log() {
	printf '[local-agent-loop] %s\n' "$*"
}

die() {
	printf '[local-agent-loop] ERROR: %s\n' "$*" >&2
	exit 1
}

require_bin() {
	command -v "$1" >/dev/null 2>&1 || die "Missing required command: $1"
}

detect_json_tool() {
	if command -v jq >/dev/null 2>&1; then
		JSON_BIN="jq"
	elif command -v jaq >/dev/null 2>&1; then
		JSON_BIN="jaq"
	else
		die "Missing required command: jq or jaq"
	fi
}

json_get() {
	"${JSON_BIN}" -r "$1" "$2"
}

json_passes() {
	"${JSON_BIN}" -e "$1" "$2" >/dev/null
}

curl_elf() {
	local method="$1"
	local url="$2"
	local body="${3:-}"
	local args=(
		-fsS
		-X "$method"
		"$url"
		-H 'content-type: application/json'
		-H "X-ELF-Tenant-Id: ${TENANT_ID}"
		-H "X-ELF-Project-Id: ${PROJECT_ID}"
		-H "X-ELF-Agent-Id: ${AGENT_ID}"
		-H "X-ELF-Read-Profile: ${READ_PROFILE}"
	)

	if [[ -n "$body" ]]; then
		args+=(--data-binary "@${body}")
	fi

	curl "${args[@]}"
}

cleanup_services() {
	if ((${#SERVICE_PIDS[@]} == 0)); then
		return
	fi

	log "Stopping background ELF services."
	for pid in "${SERVICE_PIDS[@]}"; do
		if kill -0 "$pid" >/dev/null 2>&1; then
			kill "$pid" >/dev/null 2>&1 || true
		fi
	done
}

start_dependencies() {
	require_bin docker
	log "Starting Postgres and Qdrant through docker compose."
	docker compose -f docker-compose.yml config >/dev/null
	docker compose -f docker-compose.yml up -d postgres qdrant
}

build_services() {
	require_bin cargo
	log "Building elf-api, elf-worker, and elf-mcp."
	cargo build -p elf-api -p elf-worker -p elf-mcp
}

target_dir() {
	cargo metadata --format-version 1 --no-deps | "${JSON_BIN}" -r '.target_directory'
}

start_services() {
	local target
	target="$(target_dir)"
	mkdir -p "$OUT_DIR/logs"

	log "Starting elf-api, elf-worker, and elf-mcp with ${CONFIG_PATH}."
	"${target}/debug/elf-api" -c "$CONFIG_PATH" >"${OUT_DIR}/logs/elf-api.log" 2>&1 &
	SERVICE_PIDS+=("$!")
	"${target}/debug/elf-worker" -c "$CONFIG_PATH" >"${OUT_DIR}/logs/elf-worker.log" 2>&1 &
	SERVICE_PIDS+=("$!")
	"${target}/debug/elf-mcp" -c "$CONFIG_PATH" >"${OUT_DIR}/logs/elf-mcp.log" 2>&1 &
	SERVICE_PIDS+=("$!")
}

assert_services_alive() {
	local pid
	for pid in "${SERVICE_PIDS[@]}"; do
		if ! kill -0 "$pid" >/dev/null 2>&1; then
			tail -n 80 "${OUT_DIR}"/logs/*.log >&2 || true
			die "A background ELF service exited before the demo completed."
		fi
	done
}

wait_for_health() {
	log "Waiting for ${HTTP_BASE}/health."
	for _ in $(seq 1 120); do
		if curl -fsS "${HTTP_BASE}/health" >"${OUT_DIR}/00-health.json" 2>/dev/null; then
			log "API health check passed."
			return
		fi
		assert_services_alive
		sleep 1
	done

	tail -n 80 "${OUT_DIR}"/logs/*.log >&2 || true
	die "Timed out waiting for API health."
}

write_demo_inputs() {
	local demo_id="$1"
	local source_note_id="${2:-}"
	local doc_id="${3:-}"
	local promoted_text="Fact: ELF local agent setup supports a source-linked memory review loop."

	cat >"${OUT_DIR}/01-docs-put.request.json" <<JSON
{
  "scope": "project_shared",
  "doc_type": "knowledge",
  "title": "Local agent memory loop source",
  "source_ref": {
    "schema": "doc_source_ref/v1",
    "doc_type": "knowledge",
    "ts": "2026-06-23T00:00:00Z",
    "source_kind": "text_export",
    "canonical_uri": "urn:elf:local-agent-loop",
    "captured_at": "2026-06-23T00:00:00Z",
    "trust_label": "user_captured",
    "notes": "Local deterministic setup demo source."
  },
  "content": "ELF local agent setup supports a source-linked memory review loop. The loop imports a source, proposes a memory candidate, applies reviewer approval, recalls through the agent-facing API, inspects recall debug, and restores a superseded memory when rollback is required."
}
JSON

	if [[ -n "$doc_id" ]]; then
		cat >"${OUT_DIR}/02-notes-ingest.request.json" <<JSON
{
  "scope": "agent_private",
  "notes": [
    {
      "type": "fact",
      "key": "local_agent_loop_source_${demo_id}",
      "text": "Fact: The local ELF agent loop imports sources, proposes reviewed memory, recalls it, and supports rollback.",
      "importance": 0.82,
      "confidence": 0.93,
      "ttl_days": 14,
      "source_ref": {
        "schema": "source_ref/v1",
        "resolver": "elf_doc_ext/v1",
        "ref": {
          "doc_id": "${doc_id}"
        }
      }
    }
  ]
}
JSON
	fi

	if [[ -n "$source_note_id" && -n "$doc_id" ]]; then
		cat >"${OUT_DIR}/03-consolidation-run.request.json" <<JSON
{
  "job_kind": "manual",
  "input_refs": [
    {
      "kind": "note",
      "id": "${source_note_id}",
      "snapshot": {
        "status": "active",
        "content_hash": "blake3:local-agent-loop-source",
        "embedding_version": "local:local-hash:256",
        "source_ref": {
          "schema": "source_ref/v1",
          "resolver": "elf_doc_ext/v1",
          "ref": {
            "doc_id": "${doc_id}"
          }
        },
        "metadata": {
          "demo": "local-agent-loop"
        }
      }
    }
  ],
  "source_snapshot": {
    "source_count": 1,
    "doc_id": "${doc_id}"
  },
  "lineage": {
    "source_refs": [
      {
        "kind": "note",
        "id": "${source_note_id}",
        "snapshot": {
          "status": "active",
          "content_hash": "blake3:local-agent-loop-source",
          "embedding_version": "local:local-hash:256",
          "source_ref": {
            "schema": "source_ref/v1",
            "resolver": "elf_doc_ext/v1",
            "ref": {
              "doc_id": "${doc_id}"
            }
          },
          "metadata": {
            "demo": "local-agent-loop"
          }
        }
      }
    ],
    "parent_run_id": null,
    "parent_proposal_ids": []
  },
  "proposals": [
    {
      "proposal_kind": "derived_note",
      "apply_intent": "create_derived_note",
      "source_refs": [
        {
          "kind": "note",
          "id": "${source_note_id}",
          "snapshot": {
            "status": "active",
            "content_hash": "blake3:local-agent-loop-source",
            "embedding_version": "local:local-hash:256",
            "source_ref": {
              "schema": "source_ref/v1",
              "resolver": "elf_doc_ext/v1",
              "ref": {
                "doc_id": "${doc_id}"
              }
            },
            "metadata": {
              "demo": "local-agent-loop"
            }
          }
        }
      ],
      "source_snapshot": {
        "source_count": 1,
        "doc_id": "${doc_id}"
      },
      "lineage": {
        "source_refs": [
          {
            "kind": "note",
            "id": "${source_note_id}",
            "snapshot": {
              "status": "active",
              "content_hash": "blake3:local-agent-loop-source",
              "embedding_version": "local:local-hash:256",
              "source_ref": {
                "schema": "source_ref/v1",
                "resolver": "elf_doc_ext/v1",
                "ref": {
                  "doc_id": "${doc_id}"
                }
              },
              "metadata": {
                "demo": "local-agent-loop"
              }
            }
          }
        ],
        "parent_run_id": null,
        "parent_proposal_ids": []
      },
      "confidence": 0.93,
      "unsupported_claim_flags": [],
      "markers": {
        "contradictions": [],
        "staleness": []
      },
      "diff": {
        "summary": "Promote the reviewed local setup memory without mutating source records.",
        "before": {},
        "after": {
          "target": "derived_note",
          "text": "${promoted_text}"
        }
      },
      "target_ref": {},
      "proposed_payload": {
        "type": "fact",
        "scope": "agent_private",
        "key": "local_agent_loop_promoted_${demo_id}",
        "text": "${promoted_text}",
        "importance": 0.82,
        "confidence": 0.93,
        "ttl_days": 14,
        "source_ref": {
          "schema": "source_ref/v1",
          "resolver": "elf_doc_ext/v1",
          "ref": {
            "doc_id": "${doc_id}",
            "source_note_id": "${source_note_id}"
          }
        }
      }
    }
  ]
}
JSON
	fi
}

wait_for_proposal() {
	local run_id="$1"

	for _ in $(seq 1 90); do
		if curl_elf GET "${ADMIN_BASE}/v2/admin/consolidation/proposals?run_id=${run_id}&limit=10" \
			>"${OUT_DIR}/04-proposals-list.json" 2>/dev/null &&
			json_passes '.proposals | length > 0' "${OUT_DIR}/04-proposals-list.json"; then
			json_get '.proposals[0].proposal_id' "${OUT_DIR}/04-proposals-list.json"
			return
		fi
		assert_services_alive
		sleep 1
	done

	die "Timed out waiting for consolidation proposal materialization."
}

wait_for_search_hit() {
	local note_id="$1"

	cat >"${OUT_DIR}/07-search.request.json" <<JSON
{
  "mode": "quick_find",
  "query": "source-linked memory review loop",
  "top_k": 5,
  "candidate_k": 20,
  "payload_level": "l0"
}
JSON

	for _ in $(seq 1 60); do
		if curl_elf POST "${HTTP_BASE}/v2/searches" "${OUT_DIR}/07-search.request.json" \
			>"${OUT_DIR}/07-search.response.json" 2>/dev/null &&
			json_get '.items[]?.note_id' "${OUT_DIR}/07-search.response.json" | grep -qx "$note_id"; then
			json_get '.trace_id' "${OUT_DIR}/07-search.response.json"
			return
		fi
		assert_services_alive
		sleep 1
	done

	die "Timed out waiting for promoted memory to appear in search."
}

run_demo() {
	local demo_id doc_id source_note_id run_id proposal_id promoted_note_id trace_id supersede_version

	demo_id="$(date -u '+%Y%m%d%H%M%S')"
	mkdir -p "$OUT_DIR"
	rm -f "${OUT_DIR}"/*.json

	write_demo_inputs "$demo_id"

	log "Importing a Source Library document."
	curl_elf POST "${HTTP_BASE}/v2/docs" "${OUT_DIR}/01-docs-put.request.json" \
		>"${OUT_DIR}/01-docs-put.response.json"
	doc_id="$(json_get '.doc_id' "${OUT_DIR}/01-docs-put.response.json")"

	write_demo_inputs "$demo_id" "" "$doc_id"

	log "Writing a deterministic source note linked to the imported document."
	curl_elf POST "${HTTP_BASE}/v2/notes/ingest" "${OUT_DIR}/02-notes-ingest.request.json" \
		>"${OUT_DIR}/02-notes-ingest.response.json"
	source_note_id="$(json_get '.results[0].note_id' "${OUT_DIR}/02-notes-ingest.response.json")"

	write_demo_inputs "$demo_id" "$source_note_id" "$doc_id"

	log "Creating a reviewable memory proposal."
	curl_elf POST "${ADMIN_BASE}/v2/admin/consolidation/runs" \
		"${OUT_DIR}/03-consolidation-run.request.json" \
		>"${OUT_DIR}/03-consolidation-run.response.json"
	run_id="$(json_get '.run.run_id' "${OUT_DIR}/03-consolidation-run.response.json")"
	proposal_id="$(wait_for_proposal "$run_id")"

	cat >"${OUT_DIR}/05-proposal-review.request.json" <<JSON
{
  "action": "apply",
  "review_comment": "Apply reviewed local setup memory candidate."
}
JSON

	log "Approving and applying the reviewed memory proposal."
	curl_elf POST "${ADMIN_BASE}/v2/admin/consolidation/proposals/${proposal_id}/review" \
		"${OUT_DIR}/05-proposal-review.request.json" \
		>"${OUT_DIR}/05-proposal-review.response.json"
	promoted_note_id="$(json_get '.target_ref.id' "${OUT_DIR}/05-proposal-review.response.json")"

	log "Searching through the agent-facing recall API."
	trace_id="$(wait_for_search_hit "$promoted_note_id")"

	cat >"${OUT_DIR}/08-recall-debug.request.json" <<JSON
{
  "trace_id": "${trace_id}",
  "query": "source-linked memory review loop",
  "docs_query": "source-linked memory review loop",
  "include_dreaming": true,
  "limit": 5
}
JSON

	log "Inspecting the recall debug panel."
	curl_elf POST "${HTTP_BASE}/v2/recall-debug/panel" "${OUT_DIR}/08-recall-debug.request.json" \
		>"${OUT_DIR}/08-recall-debug.response.json"

	cat >"${OUT_DIR}/09-correction-supersede.request.json" <<JSON
{
  "action": "supersede",
  "reason": "Local demo exercises correction before rollback.",
  "source_ref": {
    "schema": "local_agent_loop/review",
    "source": "supersede"
  },
  "restore_version_id": null
}
JSON

	log "Superseding the promoted memory."
	curl_elf POST "${ADMIN_BASE}/v2/admin/notes/${promoted_note_id}/corrections" \
		"${OUT_DIR}/09-correction-supersede.request.json" \
		>"${OUT_DIR}/09-correction-supersede.response.json"
	supersede_version="$(json_get '.version_id' "${OUT_DIR}/09-correction-supersede.response.json")"

	cat >"${OUT_DIR}/10-correction-restore.request.json" <<JSON
{
  "action": "restore",
  "reason": "Rollback to the prior approved memory after local demo verification.",
  "source_ref": {
    "schema": "local_agent_loop/review",
    "source": "restore"
  },
  "restore_version_id": "${supersede_version}"
}
JSON

	log "Restoring the promoted memory from the correction ledger."
	curl_elf POST "${ADMIN_BASE}/v2/admin/notes/${promoted_note_id}/corrections" \
		"${OUT_DIR}/10-correction-restore.request.json" \
		>"${OUT_DIR}/10-correction-restore.response.json"

	log "Demo complete."
	cat <<SUMMARY

Local ELF agent loop complete.
Artifacts: ${OUT_DIR}

Source document: ${doc_id}
Source note: ${source_note_id}
Consolidation run: ${run_id}
Proposal: ${proposal_id}
Promoted memory note: ${promoted_note_id}
Search trace: ${trace_id}

Agent-facing MCP equivalents:
- elf_docs_put
- elf_notes_ingest
- elf_searches_create
- elf_recall_debug_panel

Admin review/correction HTTP surfaces used:
- POST /v2/admin/consolidation/runs
- POST /v2/admin/consolidation/proposals/{proposal_id}/review
- POST /v2/admin/notes/{note_id}/corrections
SUMMARY
}

main() {
	case "$MODE" in
		-h | --help)
			usage
			exit 0
			;;
		run | demo)
			;;
		*)
			usage >&2
			die "Unknown mode: ${MODE}"
			;;
	esac

	cd "$ROOT_DIR"
	detect_json_tool
	require_bin curl

	if [[ "$MODE" == "run" ]]; then
		trap cleanup_services EXIT
		mkdir -p "$OUT_DIR"
		start_dependencies
		build_services
		start_services
		wait_for_health
	fi

	run_demo
}

main "$@"
