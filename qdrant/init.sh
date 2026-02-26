#!/usr/bin/env bash
set -euo pipefail

: "${ELF_QDRANT_HTTP_URL:?Set ELF_QDRANT_HTTP_URL to the Qdrant REST base URL, for example http://127.0.0.1:6333.}"
: "${ELF_QDRANT_COLLECTION:?Set ELF_QDRANT_COLLECTION to the collection name.}"
: "${ELF_QDRANT_VECTOR_DIM:?Set ELF_QDRANT_VECTOR_DIM to the dense vector dimension.}"

collections=("${ELF_QDRANT_COLLECTION}")

if [[ -n "${ELF_QDRANT_DOCS_COLLECTION:-}" ]]; then
  collections+=("${ELF_QDRANT_DOCS_COLLECTION}")
fi

create_payload_index() {
  local collection=$1
  local payload=$2
  local field_name
  local response
  local status
  response="$(mktemp)"
  field_name="${payload#*\"field_name\":\"}"
  field_name="${field_name%%\"*}"

  status=$(curl -sS -w '%{http_code}' -o "$response" -X PUT \
    "${ELF_QDRANT_HTTP_URL}/collections/${collection}/index?wait=true" \
    -H 'Content-Type: application/json' \
    -d "$payload"
  )

  if [[ "$status" == 2* ]]; then
    rm -f "$response"
    return
  fi

  if grep -qi "already.*exists" "$response"; then
    rm -f "$response"
    return
  fi

  echo "Failed to create payload index for field '${field_name}' in ${collection}. HTTP ${status}." >&2
  echo "Response body: $(cat "$response")" >&2
  rm -f "$response"
  exit 1
}

for collection in "${collections[@]}"; do
  collection_exists=false

  if curl -fsS "${ELF_QDRANT_HTTP_URL}/collections/${collection}" >/dev/null 2>&1; then
    echo "Qdrant collection ${collection} already exists. Skipping create."
    collection_exists=true
  fi

  if [[ "$collection_exists" == "false" ]]; then
    echo "Creating Qdrant collection ${collection}."

    curl -sS -X PUT "${ELF_QDRANT_HTTP_URL}/collections/${collection}?wait=true" \
      -H 'Content-Type: application/json' \
      -d @- <<JSON
{
  "vectors": {
    "dense": {
      "size": ${ELF_QDRANT_VECTOR_DIM},
      "distance": "Cosine"
    }
  },
  "sparse_vectors": {
    "bm25": {
      "modifier": "idf"
    }
  }
}
JSON
  fi

  if [[ -n "${ELF_QDRANT_DOCS_COLLECTION:-}" && "${collection}" == "${ELF_QDRANT_DOCS_COLLECTION}" ]]; then
    create_payload_index "$collection" '{"field_name":"scope","field_schema":"keyword"}'
    create_payload_index "$collection" '{"field_name":"status","field_schema":"keyword"}'
    create_payload_index "$collection" '{"field_name":"doc_type","field_schema":"keyword"}'
    create_payload_index "$collection" '{"field_name":"agent_id","field_schema":"keyword"}'
    create_payload_index "$collection" '{"field_name":"updated_at","field_schema":"datetime"}'
    create_payload_index "$collection" '{"field_name":"doc_ts","field_schema":"datetime"}'
    create_payload_index "$collection" '{"field_name":"thread_id","field_schema":"keyword"}'
    create_payload_index "$collection" '{"field_name":"domain","field_schema":"keyword"}'
    create_payload_index "$collection" '{"field_name":"repo","field_schema":"keyword"}'
  fi
done
