# Notification / Migration / HA readiness closure

Date: 2026-06-22
Branch: `main`
Scope: Notification Center, `tikeo-migrate`, and Raft FSOD Server HA product-readiness acceptance closure.

## Summary

The three active product surfaces already have detailed operator documentation. This report records the cross-feature acceptance closure and points maintainers to the public checklist now available in the docs site:

- English: `docs/docs/development/product-readiness-acceptance.md`
- Chinese: `docs/i18n/zh-CN/docusaurus-plugin-content-docs/current/development/product-readiness-acceptance.md`

## Acceptance status

| Area | Status | Required proof before production sign-off |
| --- | --- | --- |
| Notification Center | Documentation and local/staging checklist closed. | Real provider test-send evidence for every enabled provider family, message trace, retry/DLQ snapshot, and redaction proof. |
| Migration CLI | Review-first migration flow checklist closed. | Sample migration bundle, clean non-mutating `plan`, dry-run `apply`, staged live import, and release assets. |
| Raft FSOD Server HA | Kind/staging acceptance checklist closed. | External DB StatefulSet deployment, one scheduler, active shard ownership, durable outbox recovery, cross-pod API consistency, Worker gateway failover, and cloud-specific network validation. |

## Canonical references

- Notification user guide: `docs/docs/user-guide/notifications.md`
- Notification reference: `docs/docs/reference/notification-center.md`
- Migration guide: `docs/docs/integrations/migrating-from-legacy-schedulers.md`
- HA runbook: `docs/docs/deployment/server-ha.md`
- Kubernetes deployment: `docs/docs/deployment/kubernetes.md`
- Production deployment: `docs/docs/deployment/production.md`

## Verification commands for this documentation closure

```bash
python3 .github/tests/docs_site_contract_test.py
python3 scripts/check-source-size.py
git diff --check
```

## Remaining operational risks

1. Cloud-provider HA validation is environment-specific and remains a deployment acceptance item.
2. Real notification provider policies can differ by tenant; keep per-environment provider evidence.
3. Legacy scheduler migration cannot safely hide route/block/concurrency/script semantic differences; review-required items are intentional.
4. Release assets must be verified from the actual GitHub Release after CI uploads finish.
