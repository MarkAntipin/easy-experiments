ARG RUST_VERSION=1.95
ARG NODE_VERSION=20
ARG CARGO_CHEF_VERSION=0.1.77
ARG CARGO_BUILD_JOBS=2

# ---------- UI build ----------
FROM node:${NODE_VERSION}-alpine AS ui-builder

WORKDIR /ui

COPY ui/package.json ui/package-lock.json ./
RUN --mount=type=cache,id=easy-experiments-npm,target=/root/.npm \
    npm ci

COPY ui/ ./

RUN npm run build

# ---------- Backend dependency planning ----------
FROM rust:${RUST_VERSION}-bookworm AS backend-chef

ARG CARGO_CHEF_VERSION
ARG CARGO_BUILD_JOBS
ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}

WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        cmake \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef --version "${CARGO_CHEF_VERSION}" --locked

FROM backend-chef AS backend-planner

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo chef prepare --recipe-path recipe.json

# ---------- Backend build ----------
FROM backend-chef AS builder

COPY --from=backend-planner /usr/src/app/recipe.json recipe.json

RUN --mount=type=cache,id=easy-experiments-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=easy-experiments-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=easy-experiments-target,target=/usr/src/app/target,sharing=private \
    cargo chef cook --release --recipe-path recipe.json

COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src

RUN --mount=type=cache,id=easy-experiments-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=easy-experiments-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=easy-experiments-target,target=/usr/src/app/target,sharing=private \
    cargo build --release && \
    cp target/release/easy-experiments /usr/local/bin/easy-experiments

RUN mkdir -p /rootfs/data

# ---------- Runtime ----------
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        tini \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 65532 nonroot \
    && useradd --system --uid 65532 --gid 65532 \
        --no-create-home --shell /usr/sbin/nologin nonroot

COPY --from=builder /usr/local/bin/easy-experiments /easy-experiments
COPY --from=builder --chown=65532:65532 /rootfs/data /data
COPY --from=ui-builder /ui/dist /ui-dist

ENV APPLICATION_PORT=18200 \
    SQLITE_URL="sqlite:///data/easy-experiments.db" \
    DUCKDB_PATH="/data/easy-experiments.duckdb" \
    UI_DIST_PATH="/ui-dist" \
    LOG_FORMAT=json \
    RUST_LOG="info,sqlx=warn,h2=warn,hyper=warn,reqwest=warn"

EXPOSE 18200

USER 65532:65532

# tini kill zombies which helps DuckDB / SQLite flushes cleanly
ENTRYPOINT ["/usr/bin/tini", "--", "/easy-experiments"]
