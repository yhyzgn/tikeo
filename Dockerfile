# syntax=docker/dockerfile:1.7

FROM rust:1.95-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock rustfmt.toml ./
COPY src ./src
COPY crates ./crates
COPY proto ./proto
COPY examples ./examples

RUN cargo build --release --bin scheduler

FROM debian:bookworm-slim AS runtime

WORKDIR /app
COPY --from=builder /app/target/release/scheduler /usr/local/bin/scheduler
COPY examples ./examples

VOLUME ["/data"]
EXPOSE 9090 9091
ENTRYPOINT ["scheduler"]
CMD ["serve", "--config", "/app/examples/container.toml"]
