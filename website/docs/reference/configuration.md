---
title: Configuration reference
description: Where Tikeo configuration lives and which settings matter for first evaluation.
---

# Configuration reference

The default development configuration lives under `config/`. Public docs should keep examples small and link to committed config files instead of copying large TOML blocks.

## First local config

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
```

## Important areas

- HTTP listener address and port.
- Worker Tunnel listener address and port.
- Storage database URL.
- Transport security: HTTP TLS and Worker Tunnel TLS/mTLS.
- Script governance and release signature secret reference.
- Alert retry worker settings.
- Observability and tracing exporters.

## Safety rule

Schema changes must go through explicit SeaORM migrations. Do not document manual database mutation as a supported configuration path.
