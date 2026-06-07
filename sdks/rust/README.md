# Tikeo Rust SDKs 🦀

[🇨🇳 中文 SDK 文档](../../README.zh-CN.md#行为一致的-sdk)

Rust SDK packages are self-contained crates suitable for crates.io publishing. They must not depend
on repository-local server crates.

```bash
cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features
cargo build --manifest-path sdks/rust/tikeo/Cargo.toml --all-features
```

Current crate: [`sdks/rust/tikeo`](tikeo/README.md).
