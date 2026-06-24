# Tikeo Roadmap

Tikeo is built toward a Rust-native orchestration control plane for scheduled jobs, workflows, scripts, and multi-language workers. The roadmap is intentionally evidence-driven: features should move forward with tests, runnable examples, and operational documentation.

## Now

- Harden local and Docker-based evaluation paths.
- Keep Java, Rust, Go, Python, and Node.js worker demos runnable and honest about capabilities.
- Improve README, documentation, and first-run experience for open-source evaluators.
- Continue Web console polish for jobs, workers, workflows, scripts, RBAC, audit, and API keys.

## Next

- Production deployment hardening: Helm values, external databases, TLS/mTLS, secret references, readiness/liveness, resource sizing, and rollback docs.
- Documentation site with quick start, concepts, SDKs, deployments, architecture, and comparison guides.
- More focused examples for Worker Tunnel, normal processors, script sandbox policy, and workflow recovery.
- Release notes and changelog discipline for preview releases.

## Later

- Deeper cluster scheduling ownership and failover validation.
- Additional workflow recovery and replay scenarios.
- Migration tools and guides for teams evaluating Tikeo against XXL-Job or PowerJob.
- Expanded observability examples for OpenTelemetry, metrics, dashboards, and incident review.

## Non-goals

- Artificial star growth, fake usage claims, or misleading badges.
- Features that require business workers to expose inbound task-execution ports by default.
- Server-side execution of user code outside controlled worker runtimes.
