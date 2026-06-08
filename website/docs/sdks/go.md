---
title: Go Worker SDK
description: Verified Go SDK and Worker demo entry points.
---

# Go Worker SDK

The Go SDK lives under `sdks/go/tikeo`, and the runnable worker demo lives under `examples/go/worker-demo`.

## Verify the SDK

```bash
cd sdks/go/tikeo
go test ./... -count=1
```

## Verify the demo

```bash
cd examples/go/worker-demo
go test ./... -count=1
```

Go workers should advertise only capabilities backed by real runtime support. Unsupported sandbox runners must fail closed instead of appearing as available capabilities.
