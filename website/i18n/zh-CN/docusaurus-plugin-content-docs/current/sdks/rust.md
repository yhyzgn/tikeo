---
title: Rust Worker SDK
description: Rust SDK 与 Worker demo 的验证入口。
---

# Rust Worker SDK

Rust SDK 位于 `sdks/rust/tikeo`，可运行 Worker demo 位于 `examples/rust/worker-demo`。它是 Tikeo 最接近 Server 内核协议的原生 SDK，也是验证 Worker Tunnel、能力广告、日志与结果回传的推荐入口。

## 运行时要求

Rust SDK 当前按 README 与 CI 声明的 Rust 1.95+ 基线维护。若未来调整 Rust toolchain，必须同步 SDK 文档、demo manifest、CI setup 和徽章，避免宣传与实际构建漂移。

## 从 crates.io 安装

将 `${TIKEO_VERSION}` 替换为 README 顶部 `Rust SDK` 徽标显示的版本号。Rust 使用不带 `v` 的版本字符串。

```bash
cargo add tikeo@${TIKEO_VERSION}
```

```toml
[dependencies]
tikeo = "${TIKEO_VERSION}"
```

## 验证 SDK

```bash
cargo fmt --manifest-path sdks/rust/tikeo/Cargo.toml -- --check
cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features
```

## 运行 demo

```bash
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

live mode 下，demo 应从本地配置连接 Worker Tunnel endpoint。

## Worker 心智模型

Rust Worker 负责三件事：连接 Server tunnel，只广告真实可执行能力，并带着 Server 下发的 assignment token 回传日志和结果。这样调度、审计、stale worker fencing 才能保持一致。

## 能力广告纪律

如果 Worker 不能执行某个 processor、script backend 或 plugin capability，就不要广告它。不可用 runtime 应 fail closed，而不是伪装成可用能力。

## 生产建议

生产环境应把 Worker 独立打包，而不是塞进 Server 镜像。Worker identity 应通过 namespace、app、worker pool、labels 和 structured capabilities 表达，不要依赖临时命名约定。

## 适合场景

Rust Worker 适合对延迟、资源占用、类型安全和单二进制分发要求较高的任务。它也是验证协议细节的好入口：如果 Rust demo 能稳定完成注册、派发、日志、结果和注销，再对照其他语言 SDK，团队更容易发现 capability、scope 或 tunnel 配置差异。

评估完成后，把使用的 Server commit、SDK 版本、配置文件和演示命令记录下来，方便复现。
