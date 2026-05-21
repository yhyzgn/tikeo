# 031 — SDK directory follow-up (completed / superseded by 034)

## Final rule

```text
sdks/<language>/<sdk-name>/
examples/<language>/<demo-name>/
```

## Completion status
- Rust SDK is normalized at `sdks/rust/scheduler-worker-sdk`.
- Java SDK is normalized as a Gradle/JDK21+ multi-project under `sdks/java/<sdk-name>`.
- Root `Dockerfile` is server-only and must not copy/cache/build SDKs or demos.
- Active follow-up details live in `.prompt/034-sdk-layout-gradle-examples.md`.
