FROM python:3.11-bookworm

ARG PAGEINDEX_REVISION=f413c66fee0bfbb7291c389333f9cc1adac68d57

RUN apt-get update \
  && apt-get install -y --no-install-recommends git \
  && rm -rf /var/lib/apt/lists/*

RUN git init /opt/pageindex \
  && git -C /opt/pageindex remote add origin https://github.com/VectifyAI/PageIndex.git \
  && git -C /opt/pageindex fetch --depth=1 origin "${PAGEINDEX_REVISION}" \
  && git -C /opt/pageindex checkout --detach FETCH_HEAD \
  && test "$(git -C /opt/pageindex rev-parse HEAD)" = "${PAGEINDEX_REVISION}"

COPY config/benchmark/locks/pageindex.txt /opt/benchmark/pageindex.txt
RUN python3 -m venv /opt/pageindex-venv \
  && /opt/pageindex-venv/bin/pip install --no-cache-dir \
    --require-hashes --requirement /opt/benchmark/pageindex.txt

COPY scripts/benchmark-unit.py /opt/benchmark/benchmark-unit.py
COPY scripts/benchmark_targets /opt/benchmark/benchmark_targets

ENV PYTHONUNBUFFERED=1

CMD ["python3", "/opt/benchmark/benchmark-unit.py", "--target", "pageindex"]
