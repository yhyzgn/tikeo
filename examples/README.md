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
- `nodejs/worker-demo/` — Node.js Worker SDK demo with live tunnel support and Java/Rust/Go-compatible sandbox auto resolution.
- `python/worker-demo/` — Python Worker SDK demo with live tunnel support and Java/Rust/Go-compatible sandbox auto resolution.
- `rust/worker-demo/` — Rust Worker SDK demo with live tunnel support and fail-closed script runner registration.

Runtime configuration files belong in `config/`, not here.
