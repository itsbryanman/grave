FROM rust:1-slim AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential pkg-config ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .
RUN cargo build --release -p grave-cli --bin grave

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /graves
COPY --from=builder /build/target/release/grave /usr/local/bin/grave

ENTRYPOINT ["grave"]
