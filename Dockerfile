FROM rust:1.95-alpine AS builder

WORKDIR /usr/src/app

RUN apk add --no-cache \
    build-base \
    musl-dev \
    linux-headers \
    pkgconfig \
    openssl-dev \
    sqlite-dev

COPY . .

RUN cargo build --release --bin easy-experiments

FROM alpine:latest

RUN apk add --no-cache sqlite

WORKDIR /app

COPY --from=builder /usr/src/app/target/release/easy-experiments .
COPY --from=builder /usr/src/app/migrations /app/migrations

CMD ["./easy-experiments"]
