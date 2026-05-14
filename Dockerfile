# syntax=docker/dockerfile:1.7

ARG RUST_VERSION=1.95

FROM rust:${RUST_VERSION}-alpine AS builder

WORKDIR /usr/src/app

RUN apk add --no-cache \
    build-base \
    musl-dev \
    linux-headers \
    pkgconfig \
    sqlite-dev \
    cmake

COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/src/app/target,sharing=locked \
    cargo build --release && \
    cp target/release/easy-experiments /usr/local/bin/easy-experiments && \
    mkdir -p /rootfs/data

# ---------- Runtime ----------
FROM gcr.io/distroless/static-debian12:nonroot AS runtime

COPY --from=builder /usr/local/bin/easy-experiments /easy-experiments
COPY --from=builder --chown=65532:65532 /rootfs/data /data

# Required at runtime:
#   JWT_SECRET=<stable random secret>
#
# Auth mode:
#   Password/self-hosted mode: omit GOOGLE_CLIENT_ID. If ADMIN_EMAIL and
#   ADMIN_PASSWORD are set, the first startup on an empty /data volume creates
#   that admin account.
#   Google mode: set GOOGLE_CLIENT_ID. Password auth is disabled in this mode.
#
# Persist /data with a Docker volume or bind mount; it contains both SQLite
# metadata and DuckDB analytics events.
ENV APPLICATION_PORT=18200 \
    SQLITE_URL="sqlite:///data/easy-experiments.db" \
    DUCKDB_PATH="/data/easy-experiments.duckdb" \
    LOG_FORMAT=json \
    RUST_LOG="info,sqlx=warn,h2=warn,hyper=warn,reqwest=warn"

EXPOSE 18200

VOLUME ["/data"]

USER 65532:65532

ENTRYPOINT ["/easy-experiments"]
