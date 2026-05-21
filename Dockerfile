# syntax=docker/dockerfile:1.7

FROM rust:1.95-alpine AS dependencies

RUN sed -i 's@dl-cdn.alpinelinux.org@mirrors.aliyun.com@g' /etc/apk/repositories \
    && apk add --no-cache build-base ca-certificates cmake perl pkgconf

# rsproxy 源配置
ENV RUSTUP_DIST_SERVER="https://rsproxy.cn"
ENV RUSTUP_UPDATE_ROOT="https://rsproxy.cn/rustup"

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
WORKDIR /app

# 复制 Cargo 配置文件
COPY .cargo/config.toml .cargo/config.toml

COPY Cargo.toml Cargo.lock rustfmt.toml ./
# Server image intentionally excludes ./sdks; keep Docker workspace server-only.
RUN perl -0pi -e 's/, \"sdks\/rust\/scheduler-worker-sdk\"//' Cargo.toml
COPY crates/scheduler-config/Cargo.toml crates/scheduler-config/Cargo.toml
COPY crates/scheduler-core/Cargo.toml crates/scheduler-core/Cargo.toml
COPY crates/scheduler-proto/Cargo.toml crates/scheduler-proto/Cargo.toml
COPY crates/scheduler-proto/build.rs crates/scheduler-proto/build.rs
COPY crates/scheduler-proto/proto crates/scheduler-proto/proto
COPY crates/scheduler-server/Cargo.toml crates/scheduler-server/Cargo.toml
COPY crates/scheduler-storage/Cargo.toml crates/scheduler-storage/Cargo.toml

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
    cargo build --release --locked --bin scheduler \
    && cp /app/target/release/scheduler /tmp/scheduler

FROM alpine:3.22 AS runtime

RUN sed -i 's@dl-cdn.alpinelinux.org@mirrors.aliyun.com@g' /etc/apk/repositories \
    && apk add --no-cache ca-certificates tzdata \
    && ln -sf /usr/share/zoneinfo/Asia/Shanghai /etc/localtime

ENV TZ=Asia/Shanghai
WORKDIR /app
COPY --from=builder /tmp/scheduler /usr/local/bin/scheduler
COPY config ./config

VOLUME ["/data"]
EXPOSE 9090 9998
ENTRYPOINT ["scheduler"]
CMD ["serve", "--config", "/app/config/container.toml"]
