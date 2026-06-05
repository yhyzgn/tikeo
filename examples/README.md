# SDK Examples

This directory contains independently buildable/runnable SDK demo projects.

Required layout:

```text
examples/<language>/<demo-name>/
```

Current runnable demos:

- `java/spring-boot2-worker-demo/` — Spring Boot 2 Worker demo with live tunnel support.
- `java/spring-boot3-worker-demo/` — Spring Boot 3 Worker demo with live tunnel support.
- `java/spring-boot4-worker-demo/` — Spring Boot 4 Worker demo with live tunnel support.
- `go/worker-demo/` — Go Worker SDK demo with live tunnel support and fail-closed script runner registration.
- `rust/worker-demo/` — Rust Worker SDK demo with live tunnel support and fail-closed script runner registration.

Planned language demo placeholders remain under `python/` and `nodejs/`; when the corresponding SDK is implemented, each placeholder must become a real standalone build/run project in the same `examples/<language>/<demo-name>/` shape.

Runtime configuration files belong in `config/`, not here.
