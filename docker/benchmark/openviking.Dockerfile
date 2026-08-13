# syntax=docker/dockerfile:1.7

FROM rust:1.91.1-trixie AS rust-toolchain

FROM ghcr.io/astral-sh/uv:python3.13-trixie-slim AS builder

ARG OPENVIKING_REVISION=a0e822a0cd5d02e8ed5330a7f7a6d13279f8308b

COPY --from=rust-toolchain /usr/local/cargo /usr/local/cargo
COPY --from=rust-toolchain /usr/local/rustup /usr/local/rustup
COPY --from=node:24-trixie-slim /usr/local/bin/node /usr/local/bin/
COPY --from=node:24-trixie-slim /usr/local/lib/node_modules/ /usr/local/lib/node_modules/

RUN ln -s ../lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm \
  && ln -s ../lib/node_modules/npm/bin/npx-cli.js /usr/local/bin/npx \
  && apt-get update \
  && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    git \
  && rm -rf /var/lib/apt/lists/*

ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV PATH="/usr/local/cargo/bin:${PATH}"
ENV SETUPTOOLS_SCM_PRETEND_VERSION_FOR_OPENVIKING=0.0.0+a0e822a
ENV OPENVIKING_VERSION=0.0.0+a0e822a
ENV UV_COMPILE_BYTECODE=1
ENV UV_LINK_MODE=copy
ENV UV_NO_DEV=1

RUN git init /opt/openviking \
  && git -C /opt/openviking remote add origin https://github.com/volcengine/OpenViking.git \
  && git -C /opt/openviking fetch --depth=1 origin "${OPENVIKING_REVISION}" \
  && git -C /opt/openviking checkout --detach FETCH_HEAD \
  && test "$(git -C /opt/openviking rev-parse HEAD)" = "${OPENVIKING_REVISION}"

WORKDIR /opt/openviking
RUN --mount=type=cache,target=/root/.cache/uv \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/opt/openviking/target \
    uv sync --frozen --no-editable \
  && git -C /opt/openviking rev-parse HEAD > /opt/openviking/REVISION

FROM python:3.13-slim-trixie

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    git \
    libstdc++6 \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /opt/openviking/REVISION /opt/openviking/REVISION
COPY --from=builder /opt/openviking/.venv /opt/openviking/.venv
COPY scripts/benchmark-unit.py /opt/benchmark/benchmark-unit.py
COPY scripts/benchmark_targets /opt/benchmark/benchmark_targets

ENV PATH="/opt/openviking/.venv/bin:${PATH}"
ENV PYTHONUNBUFFERED=1

CMD ["python3", "/opt/benchmark/benchmark-unit.py", "--target", "openviking"]
