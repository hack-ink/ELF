FROM python:3.11-bookworm

COPY config/benchmark/locks/graphiti.txt /opt/benchmark/locks/graphiti.txt
RUN python3 -m venv /opt/benchmark-venv \
  && /opt/benchmark-venv/bin/pip install --no-cache-dir \
    --require-hashes --requirement /opt/benchmark/locks/graphiti.txt

COPY scripts/benchmark-unit.py /opt/benchmark/benchmark-unit.py
COPY scripts/benchmark_targets /opt/benchmark/benchmark_targets

ENV PATH="/opt/benchmark-venv/bin:${PATH}"
ENV PYTHONUNBUFFERED=1

CMD ["python3", "/opt/benchmark/benchmark-unit.py", "--target", "graphiti"]
