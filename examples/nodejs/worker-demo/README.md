# Node.js Worker demo

Node.js Worker demo aligned one-to-one with the Rust/Go/Java manual acceptance scopes.

```bash
cd examples/nodejs/worker-demo
bun install
TIKEE_WORKER_DRY_RUN=1 bun start
bun test
```

Defaults match other demos: `dev-alpha/orders`, stable `nodejs-worker-demo-local`, SQL plugin processor `billing.sql-sync`, and script runners for shell, Python, JavaScript, TypeScript, PowerShell, PHP, Groovy, and Rhai using `auto` (`srt`/`deno`) sandbox resolution.
