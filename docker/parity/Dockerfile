FROM rust:1-bookworm

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    bash \
    ca-certificates \
    clang \
    cmake \
    curl \
    git \
    jq \
    libssl-dev \
    perl \
    pkg-config \
    postgresql-client \
    protobuf-compiler \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY . /workspace

CMD ["bash", "scripts/parity-docker-gate.sh"]
