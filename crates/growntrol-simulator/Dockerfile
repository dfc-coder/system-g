FROM rust:1.85-bookworm AS builder
WORKDIR /workspace

COPY Cargo.toml Cargo.lock ./
COPY crates/growntrol-core crates/growntrol-core
COPY crates/growntrol-protocol crates/growntrol-protocol
COPY crates/growntrol-backend crates/growntrol-backend
COPY crates/growntrol-simulator crates/growntrol-simulator

RUN cargo build --release -p growntrol-simulator

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /workspace/target/release/growntrol-simulator /usr/local/bin/growntrol-simulator
ENTRYPOINT ["growntrol-simulator"]
