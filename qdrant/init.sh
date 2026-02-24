#!/usr/bin/env bash
set -euo pipefail

: "${ELF_QDRANT_HTTP_URL:?Set ELF_QDRANT_HTTP_URL to the Qdrant REST base URL, for example http://127.0.0.1:6333.}"
: "${ELF_QDRANT_COLLECTION:?Set ELF_QDRANT_COLLECTION to the collection name.}"
: "${ELF_QDRANT_VECTOR_DIM:?Set ELF_QDRANT_VECTOR_DIM to the dense vector dimension.}"

collections=("${ELF_QDRANT_COLLECTION}")

if [[ -n "${ELF_QDRANT_DOCS_COLLECTION:-}" ]]; then
  collections+=("${ELF_QDRANT_DOCS_COLLECTION}")
fi

for collection in "${collections[@]}"; do
  if curl -fsS "${ELF_QDRANT_HTTP_URL}/collections/${collection}" >/dev/null 2>&1; then
    echo "Qdrant collection ${collection} already exists. Skipping create."
    continue
  fi

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
done
