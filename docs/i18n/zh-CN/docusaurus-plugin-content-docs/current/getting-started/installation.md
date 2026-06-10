---
title: 安装要求
description: 评估 Tikeo 所需的本地工具链、版本边界与首次检查命令。
---

# 安装要求

本页用于准备本地评估环境。文档站面向公开评估，因此命令必须指向仓库中真实存在、可以验证的入口，而不是只用于营销的伪示例。

## 必需工具

| 模块 | 运行时 / 工具链 |
|---|---|
| Server | Rust 1.95+ |
| Web 控制台 | Bun + 现代 Node 兼容环境 |
| Java SDK/demo | Java 17+ runtime；仓库构建使用 Java 21 toolchain |
| Go SDK/demo | Go 1.26+ |
| Python SDK/demo | Python 3.11+ |
| Node.js SDK/demo | Bun / Node.js 24+ CI surface |

## 克隆仓库

```bash
git clone https://github.com/yhyzgn/tikeo.git
cd tikeo
```

## 验证工具链

```bash
cargo --version
bun --version
go version
java -version
python --version
```

如果某个语言 SDK 不是本次评估范围，可以暂时跳过对应工具链；但在对外宣传“多语言 Worker”之前，至少应该运行一个真实 Worker demo。

## 推荐首次检查

```bash
cargo test --workspace --all-features
bun run --cwd web test
```

更完整的本地基线包括：

```bash
cargo fmt --all -- --check
cargo test --workspace --all-features
bun run --cwd web typecheck
bun run --cwd web test
```

## 文档站检查

文档站是独立 Docusaurus 应用，位于 `docs/`：

```bash
cd docs
bun install --frozen-lockfile
bun run docs:typecheck
bun run docs:build
```

## 常见安装误区

- 在 Server 的 Worker Tunnel 监听端口 `9998` 可用前启动 Worker。
- 使用陈旧数据库结构，而不是让 Tikeo 走正常 migration 路径。
- 在本地 Web 开发时误以为前端资源已经嵌入 Server 二进制。
- 把 Python 或 Node.js 示例当成占位内容；如果文档宣传它们，就必须能追溯到 CI 或本地验证命令。

## 下一步选择

想最快看效果，继续 [快速开始](./quickstart)。想验证镜像与数据库 overlay，阅读 [Docker Compose](../deployment/docker-compose)。想评估生产部署边界，阅读 [Kubernetes 与 Helm](../deployment/kubernetes)。
