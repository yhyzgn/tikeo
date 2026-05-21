# SDK Examples

This directory contains independently buildable/runnable SDK demo projects.

Required layout:

```text
examples/<language>/<demo-name>/
```

Current runnable demos:

- `rust/worker-demo/` — dry-run Rust Worker SDK configuration smoke test.
- `java/spring-worker-demo/` — Spring Boot starter wiring smoke test.

Planned language demo placeholders remain under `go/`, `python/`, and `nodejs/`; when the corresponding SDK is implemented, each placeholder must become a real standalone build/run project in the same `examples/<language>/<demo-name>/` shape.

Runtime configuration files belong in `config/`, not here.
