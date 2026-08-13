FROM node:22-bookworm-slim

ARG SAG_REVISION=84a5b8c9bd45944b8a3cd76e0ba4762ce68ccc72

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates git python3 \
  && rm -rf /var/lib/apt/lists/*

RUN git init /opt/sag \
  && git -C /opt/sag remote add origin https://github.com/Zleap-AI/SAG.git \
  && git -C /opt/sag fetch --depth=1 origin "${SAG_REVISION}" \
  && git -C /opt/sag checkout --detach FETCH_HEAD \
  && test "$(git -C /opt/sag rev-parse HEAD)" = "${SAG_REVISION}"

WORKDIR /opt/sag
RUN npm ci \
  && npx tsc -p tsconfig.build.json --noEmit

COPY scripts/benchmark-unit.py /opt/benchmark/benchmark-unit.py
COPY scripts/benchmark_targets /opt/benchmark/benchmark_targets

ENV SAG_REPO_DIR=/opt/sag
ENV SAG_TSX=/opt/sag/node_modules/.bin/tsx
ENV PYTHONUNBUFFERED=1

CMD ["python3", "/opt/benchmark/benchmark-unit.py", "--target", "sag"]
