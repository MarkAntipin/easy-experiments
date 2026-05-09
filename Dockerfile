# syntax=docker/dockerfile:1.7

FROM rust:1.95-alpine AS builder

WORKDIR /usr/src/app

RUN apk add --no-cache \
    build-base \
    musl-dev \
    linux-headers \
    pkgconfig \
    sqlite-dev \
    cmake

COPY . .

# Single static musl binary. `--bin easy-experiments` keeps helper bins like
# `seed_loadtest` out of the production image; `--locked` honors Cargo.lock so
# the image is bit-for-bit reproducible from a given commit.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/src/app/target,sharing=locked \
    cargo build --release --locked --bin easy-experiments && \
    cp target/release/easy-experiments /usr/local/bin/easy-experiments && \
    mkdir -p /rootfs/data

# ---------- Runtime ----------
FROM gcr.io/distroless/static-debian12:nonroot

COPY --from=builder /usr/local/bin/easy-experiments /easy-experiments
COPY --from=builder --chown=65532:65532 /rootfs/data /data

ENV APPLICATION_PORT=18200 \
    DATABASE_URL="sqlite:///data/easy-experiments.db" \
    DUCKDB_PATH="/data/easy-experiments.duckdb" \
    LOG_FORMAT=json

EXPOSE 18200

VOLUME ["/data"]

ENTRYPOINT ["/easy-experiments"]
