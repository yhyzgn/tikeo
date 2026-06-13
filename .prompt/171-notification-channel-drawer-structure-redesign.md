# Notification channel drawer structural redesign

## Goal
The previous channel drawer still felt like stacked cards without a clear hierarchy. Redesign it for local UI acceptance: operators should immediately understand the configuration map, replacement boundaries, scope cascade, connection settings, message shape, and advanced escape hatch.

## Design direction
- Left rail = live configuration map, scope ladder, save impact, and saved-config test entry.
- Main body = four domain panels:
  1. Identity and scope
  2. Connection configuration
  3. Message shape
  4. Governance and extension
- Replacement switches live only in their owning domain/subpanel.
- Advanced JSON is collapsed by default and treated as an escape hatch, not the main path.
- Preserve existing payload behavior and sensitive-data boundaries.

## Validation target
- Web typecheck, lint, notification drawer tests, source-size check, and build must pass locally.
