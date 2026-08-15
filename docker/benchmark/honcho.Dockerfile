FROM python:3.13-slim-bookworm

ARG HONCHO_REVISION=93dcf59c4a4225bb020b20c628799737fefae318

COPY --from=ghcr.io/astral-sh/uv:0.9.24 /uv /bin/uv

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates git \
  && rm -rf /var/lib/apt/lists/*

RUN git init /app \
  && git -C /app remote add origin https://github.com/plastic-labs/honcho.git \
  && git -C /app fetch --depth=1 origin "${HONCHO_REVISION}" \
  && git -C /app checkout --detach FETCH_HEAD \
  && test "$(git -C /app rev-parse HEAD)" = "${HONCHO_REVISION}" \
  && git -C /app rev-parse HEAD > /app/REVISION

WORKDIR /app
ENV UV_COMPILE_BYTECODE=1
ENV UV_LINK_MODE=copy
ENV UV_PYTHON_INSTALL_DIR=/opt/uv-python
RUN --mount=type=cache,target=/root/.cache/uv \
  uv sync --frozen --no-group dev

COPY scripts/benchmark-unit.py /opt/benchmark/benchmark-unit.py
COPY scripts/benchmark_targets /opt/benchmark/benchmark_targets

RUN addgroup --system app \
  && adduser --system --group app \
  && mkdir -p /tmp/uv-cache \
  && chown -R app:app /app /tmp/uv-cache /opt/benchmark

ENV HOME=/app
ENV HONCHO_REPO_DIR=/app
ENV PATH="/app/.venv/bin:${PATH}"
ENV PYTHONPATH="/app/sdks/python/src"
ENV PYTHONUNBUFFERED=1
ENV UV_CACHE_DIR=/tmp/uv-cache

USER app
CMD ["python3", "/opt/benchmark/benchmark-unit.py", "--target", "honcho"]
