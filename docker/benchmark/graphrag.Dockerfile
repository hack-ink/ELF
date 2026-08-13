FROM python:3.11-bookworm

COPY config/benchmark/locks/graphrag.txt /opt/benchmark/graphrag.txt
RUN python3 -m venv /opt/graphrag-venv \
  && /opt/graphrag-venv/bin/pip install --no-cache-dir \
    --require-hashes --requirement /opt/benchmark/graphrag.txt \
  && /opt/graphrag-venv/bin/python -c \
    "import importlib.metadata as m; assert m.version('graphrag') == '3.1.0'"

COPY scripts/benchmark-unit.py /opt/benchmark/benchmark-unit.py
COPY scripts/benchmark_targets /opt/benchmark/benchmark_targets

ENV GRAPHRAG_EXECUTABLE=/opt/graphrag-venv/bin/graphrag
ENV PYTHONUNBUFFERED=1

CMD ["/opt/graphrag-venv/bin/python", "/opt/benchmark/benchmark-unit.py", "--target", "graphrag"]
