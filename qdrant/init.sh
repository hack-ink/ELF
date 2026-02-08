#!/usr/bin/env bash
set -euo pipefail

: "${ELF_QDRANT_HTTP_URL:?Set ELF_QDRANT_HTTP_URL to the Qdrant REST base URL, for example http://127.0.0.1:6333.}"
: "${ELF_QDRANT_COLLECTION:?Set ELF_QDRANT_COLLECTION to the collection name.}"
: "${ELF_QDRANT_VECTOR_DIM:?Set ELF_QDRANT_VECTOR_DIM to the dense vector dimension.}"

if curl -fsS "${ELF_QDRANT_HTTP_URL}/collections/${ELF_QDRANT_COLLECTION}" >/dev/null 2>&1; then
  echo "Qdrant collection ${ELF_QDRANT_COLLECTION} already exists. Skipping create."
  exit 0
fi

curl -sS -X PUT "${ELF_QDRANT_HTTP_URL}/collections/${ELF_QDRANT_COLLECTION}?wait=true" \
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
