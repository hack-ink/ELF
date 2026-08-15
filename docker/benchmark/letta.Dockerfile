FROM letta/letta@sha256:aa66c3eeee13d2dfc40c650d709b550237ee31bfc91942a52fa488a13fa8c102

COPY config/benchmark/locks/letta.txt /opt/benchmark/locks/letta.txt
RUN python3 -m venv /opt/benchmark-venv \
  && /opt/benchmark-venv/bin/pip install --no-cache-dir \
    --require-hashes --requirement /opt/benchmark/locks/letta.txt

COPY scripts/benchmark-unit.py /opt/benchmark/benchmark-unit.py
COPY scripts/benchmark_targets /opt/benchmark/benchmark_targets

ENV PATH="/opt/benchmark-venv/bin:/app/.venv/bin:${PATH}"
ENV LETTA_BASE_URL="http://127.0.0.1:8283"
ENV PYTHONUNBUFFERED=1

CMD ["sh", "-c", "mkdir -p /benchmark/artifacts/raw; export OPENAI_API_KEY=\"${EMBEDDING_API_KEY}\"; /app/letta/server/startup.sh > /benchmark/artifacts/raw/letta-server.log 2>&1 & server_pid=$!; trap 'kill -INT ${server_pid} 2>/dev/null || true' EXIT; python3 /opt/benchmark/benchmark-unit.py --target letta; status=$?; kill -INT ${server_pid} 2>/dev/null || true; wait ${server_pid} 2>/dev/null || true; exit ${status}"]
