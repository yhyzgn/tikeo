# syntax=docker/dockerfile:1.7

FROM rust:1.95-bookworm AS dependencies

RUN sed -i 's@deb.debian.org@mirrors.aliyun.com@g' /etc/apt/sources.list.d/debian.sources \
    && apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates musl-tools pkg-config perl \
    && rustup target add x86_64-unknown-linux-musl \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/cache/apt/*

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
WORKDIR /app

COPY Cargo.toml Cargo.lock rustfmt.toml ./
COPY crates/scheduler-config/Cargo.toml crates/scheduler-config/Cargo.toml
COPY crates/scheduler-core/Cargo.toml crates/scheduler-core/Cargo.toml
COPY crates/scheduler-proto/Cargo.toml crates/scheduler-proto/Cargo.toml
COPY crates/scheduler-proto/build.rs crates/scheduler-proto/build.rs
COPY crates/scheduler-proto/proto crates/scheduler-proto/proto
COPY crates/scheduler-server/Cargo.toml crates/scheduler-server/Cargo.toml
COPY crates/scheduler-storage/Cargo.toml crates/scheduler-storage/Cargo.toml
COPY crates/scheduler-worker-sdk/Cargo.toml crates/scheduler-worker-sdk/Cargo.toml

RUN mkdir -p src \
    && echo 'fn main() {}' > src/main.rs \
    && for crate in crates/*; do mkdir -p "${crate}/src"; echo 'pub fn placeholder() {}' > "${crate}/src/lib.rs"; done

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo fetch --locked --target x86_64-unknown-linux-musl

FROM dependencies AS builder

COPY src ./src
COPY crates ./crates
COPY proto ./proto
COPY examples ./examples

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked --target x86_64-unknown-linux-musl --bin scheduler \
    && cp /app/target/x86_64-unknown-linux-musl/release/scheduler /tmp/scheduler

FROM alpine:3.22 AS runtime

RUN sed -i 's@dl-cdn.alpinelinux.org@mirrors.aliyun.com@g' /etc/apk/repositories \
    && apk add --no-cache ca-certificates tzdata \
    && ln -sf /usr/share/zoneinfo/Asia/Shanghai /etc/localtime

ENV TZ=Asia/Shanghai
WORKDIR /app
COPY --from=builder /tmp/scheduler /usr/local/bin/scheduler
COPY examples ./examples

VOLUME ["/data"]
EXPOSE 9090 9091
ENTRYPOINT ["scheduler"]
CMD ["serve", "--config", "/app/examples/container.toml"]
