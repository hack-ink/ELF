# syntax=docker/dockerfile:1.7

FROM rust:1-bookworm AS elf-builder

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    clang \
    cmake \
    libssl-dev \
    pkg-config \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs rust-toolchain.toml ./
COPY apps ./apps
COPY packages ./packages
COPY config ./config
COPY sql ./sql

RUN --mount=type=cache,id=elf-benchmark-cargo-registry,target=/usr/local/cargo/registry \
  --mount=type=cache,id=elf-benchmark-cargo-git,target=/usr/local/cargo/git \
  --mount=type=cache,id=elf-benchmark-cargo-target,target=/src/target \
  cargo build --locked --release -p elf-eval --bin real_world_live_adapter \
  && install -D /src/target/release/real_world_live_adapter \
    /out/real_world_live_adapter

FROM node:22-bookworm AS qmd-builder

ARG QMD_REVISION=e428df76bc0274d9e93eb7ca3e95673315c42e90

RUN apt-get update \
  && apt-get install -y --no-install-recommends git \
  && rm -rf /var/lib/apt/lists/* \
  && npm install --global pnpm@10.17.1

RUN git init /opt/qmd \
  && git -C /opt/qmd remote add origin https://github.com/tobi/qmd.git \
  && git -C /opt/qmd fetch --depth=1 origin "${QMD_REVISION}" \
  && git -C /opt/qmd checkout --detach FETCH_HEAD

WORKDIR /opt/qmd
RUN pnpm install --frozen-lockfile \
  && pnpm run build

FROM node:22-bookworm

ARG ELF_SOURCE_COMMIT=unknown

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    ca-certificates \
    git \
    python3 \
    python3-pip \
  && rm -rf /var/lib/apt/lists/*

COPY config/benchmark/locks/mem0.txt /opt/benchmark/locks/mem0.txt
RUN python3 -m pip install --break-system-packages --no-cache-dir \
  --require-hashes --requirement /opt/benchmark/locks/mem0.txt

COPY --from=elf-builder /out/real_world_live_adapter \
  /usr/local/bin/real_world_live_adapter
COPY --from=qmd-builder /opt/qmd /opt/qmd
COPY config/local/elf.docker.toml config/local/tokenizer.wordlevel.json /opt/elf/
COPY scripts/benchmark-unit.py /opt/benchmark/benchmark-unit.py
COPY scripts/benchmark-provider-preflight.py /opt/benchmark/benchmark-provider-preflight.py
COPY scripts/benchmark_targets /opt/benchmark/benchmark_targets

LABEL org.opencontainers.image.revision="${ELF_SOURCE_COMMIT}"

ENV MEM0_TELEMETRY=false
ENV PYTHONUNBUFFERED=1

CMD ["python3", "/opt/benchmark/benchmark-unit.py", "--help"]
