FROM ghcr.io/astral-sh/uv@sha256:9874eb7afe5ca16c363fe80b294fe700e460df29a55532bbfea234a0f12eddb1 AS uv

FROM python:3.11-bookworm

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates git \
  && rm -rf /var/lib/apt/lists/*

COPY --from=uv /uv /uvx /usr/local/bin/

RUN git init /opt/openkb \
  && git -C /opt/openkb remote add origin https://github.com/VectifyAI/OpenKB.git \
  && git -C /opt/openkb fetch --depth=1 origin 0d905e40afa6f416c616f00c56a4cd0fd995a787 \
  && git -C /opt/openkb checkout --detach FETCH_HEAD \
  && test "$(git -C /opt/openkb rev-parse HEAD)" = 0d905e40afa6f416c616f00c56a4cd0fd995a787 \
  && UV_PROJECT_ENVIRONMENT=/opt/openkb-venv \
    uv sync --frozen --no-dev --project /opt/openkb

COPY scripts/benchmark-unit.py /opt/benchmark/benchmark-unit.py
COPY scripts/benchmark_targets /opt/benchmark/benchmark_targets

ENV OPENKB_PYTHON=/opt/openkb-venv/bin/python
ENV OPENKB_REPO_DIR=/opt/openkb
ENV PYTHONUNBUFFERED=1

CMD ["python3", "/opt/benchmark/benchmark-unit.py", "--target", "openkb"]
