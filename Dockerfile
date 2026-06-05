FROM rust:alpine AS dependencies

RUN apk update --force-missing-repositories \
    && apk upgrade \
    && apk add --no-cache build-base ca-certificates cmake perl pkgconf protobuf-dev gcompat

# Rust official distribution and sparse registry configuration.
ENV RUSTUP_DIST_SERVER="https://static.rust-lang.org"
ENV RUSTUP_UPDATE_ROOT="https://static.rust-lang.org/rustup"
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

WORKDIR /app

COPY Cargo.toml Cargo.lock rustfmt.toml ./
# Server image intentionally excludes ./sdks; keep Docker workspace server-only.
COPY crates/tikee-config/Cargo.toml crates/tikee-config/Cargo.toml
COPY crates/tikee-core/Cargo.toml crates/tikee-core/Cargo.toml
COPY crates/tikee-proto/Cargo.toml crates/tikee-proto/Cargo.toml
COPY crates/tikee-proto/build.rs crates/tikee-proto/build.rs
COPY crates/tikee-proto/proto crates/tikee-proto/proto
COPY crates/tikee-server/Cargo.toml crates/tikee-server/Cargo.toml
COPY crates/tikee-storage/Cargo.toml crates/tikee-storage/Cargo.toml
COPY crates/tikee-wasm/Cargo.toml crates/tikee-wasm/Cargo.toml

RUN mkdir -p src \
    && echo 'fn main() {}' > src/main.rs \
    && for crate in crates/*; do mkdir -p "${crate}/src"; echo 'pub fn placeholder() {}' > "${crate}/src/lib.rs"; done

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo fetch --locked

FROM dependencies AS builder

COPY src ./src
COPY crates ./crates
COPY proto ./proto
COPY config ./config

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked --bin tikee \
    && cp /app/target/release/tikee /tmp/tikee

FROM alpine:latest AS runtime

RUN apk update --force-missing-repositories \
    && apk upgrade \
    && apk add --no-cache bash openssl-dev curl ca-certificates tzdata busybox-extras \
    && rm -rf /tmp/* /var/cache/apk/* \
    && ln -sf /usr/share/zoneinfo/Asia/Shanghai /etc/localtime

ENV TZ=Asia/Shanghai
ENV MIMALLOC_PURGE_DELAY=0
ENV MIMALLOC_PURGE_DECOMMITS=1
ENV MIMALLOC_ABANDONED_PAGE_PURGE=1
LABEL maintainer="Neo<yhyzgn@gmail.com>"

WORKDIR /app
COPY --from=builder /tmp/tikee /usr/local/bin/tikee
COPY config ./config

VOLUME ["/data"]
EXPOSE 9090 9998
ENTRYPOINT ["tikee"]
CMD ["serve", "--config", "/app/config/container.toml"]
