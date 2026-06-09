---
title: Node.js Worker SDK
description: Verified Node.js SDK and Bun-powered Worker demo entry points.
---

# Node.js Worker SDK

The Node.js SDK lives under `sdks/nodejs/tikeo`, and the runnable worker demo lives under `examples/nodejs/worker-demo`. Repository scripts use Bun by default, while the published package declares a Node.js runtime baseline for consumers.

## Runtime requirement

The SDK package declares `engines.node >=24.0.0`. Bun is the repository package runner for install, tests, build, and demo execution. Keep `package.json`, README badges, docs, and CI runtime policy aligned whenever this baseline changes.


## Install from npm

Replace `${TIKEO_VERSION}` with the version shown by the top README `Node.js SDK` badge. npm uses the plain version string without a leading `v`.

```bash
bun add @yhyzgn/tikeo@${TIKEO_VERSION}
npm install @yhyzgn/tikeo@${TIKEO_VERSION}
pnpm add @yhyzgn/tikeo@${TIKEO_VERSION}
```

```ts
import { Client, WorkerConfig } from "@yhyzgn/tikeo";
```

## Verify the SDK

```bash
cd sdks/nodejs/tikeo
bun install
bun test
bun run build
```

The build emits `dist/index.js`, TypeScript declarations, and the Worker Tunnel protobuf asset copied into `dist/proto`.

## Verify the demo

```bash
cd examples/nodejs/worker-demo
bun install
TIKEO_WORKER_DRY_RUN=1 bun start
bun test
```

Dry-run mode proves local package linkage and capability metadata without requiring a live Server.

## Live-mode expectations

Live mode defaults to `http://127.0.0.1:9998` and the development scope used by the demo. The demo includes JS/TS-friendly processor behavior plus sandbox runner auto-resolution for configured script paths. Start the Server before running live mode and confirm the Worker appears in the Web console.

## Capability discipline

JavaScript and TypeScript workers can integrate quickly with web ecosystems, but they must still follow Tikeo's scheduling contract. Do not advertise SQL, script, plugin, or processor capabilities unless the runtime can execute them safely. Missing tools should fail closed and surface visible task or diagnostic errors.

## Evaluation checklist

- Run `bun test` and `bun run build` in the SDK package.
- Run the worker demo in dry-run mode.
- Run live mode against a local Server to verify Worker Tunnel behavior.
- Trigger a job mapped to the Node.js processor surface.
- Inspect logs, result payload, worker session, and audit evidence.

## Production notes

Prefer minimal container images with Bun or a Node.js 24+ runtime selected deliberately. Do not mix npm/yarn lockfiles into this repository path unless release tooling explicitly introduces them.
