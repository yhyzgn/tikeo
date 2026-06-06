# Python Worker demo

Python Worker demo aligned one-to-one with the Rust/Go/Java manual acceptance scopes.

```bash
cd examples/python/worker-demo
python -m pip install -e ../../../sdks/python/tikee
python -m pip install -e .
TIKEE_WORKER_DRY_RUN=1 python -m tikee_python_worker_demo
```

Defaults match the Go/Rust demos: `dev-alpha/orders`, stable `python-worker-demo-local`, SQL plugin processor `billing.sql-sync`, and script runners for shell, Python, JavaScript, TypeScript, PowerShell, PHP, Groovy, and Rhai using `auto` (`srt`/`deno`) sandbox resolution.
