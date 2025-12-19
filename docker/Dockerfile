ARG RUST_VERSION=1.90
ARG BIN=c2-api

FROM rust:${RUST_VERSION}-slim-bookworm AS builder
ARG BIN

WORKDIR /app

RUN mkdir -p /app/bin

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        clang \
        cmake \
        pkg-config \
        libssl-dev \
        libzmq3-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY services ./services

RUN --mount=type=cache,id=c2-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=c2-cargo-target-bookworm,target=/app/target \
    cargo build --release -p ${BIN} \
    && cp /app/target/release/${BIN} /app/bin/${BIN}

FROM debian:bookworm-slim AS runtime
ARG BIN

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        libzmq5 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -u 10001 -g root -s /sbin/nologin c2 \
    && mkdir -p /app/bin /app/templates /var/lib/c2 \
    && chown -R 10001:0 /app /var/lib/c2

WORKDIR /app

COPY --from=builder /app/bin/${BIN} /app/bin/app
COPY --from=builder /app/services/c2-web/templates /app/templates

ENV C2_DATA_DIR=/var/lib/c2
ENV C2_WEB_TEMPLATES_DIR=/app/templates

EXPOSE 8080

USER 10001

ENTRYPOINT ["/app/bin/app"]
