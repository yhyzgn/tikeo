# `.prompt` next-phase handoff prompts

This directory contains takeover prompts for future development phases.

Rules:

- Use a three-digit prefix plus a phase name, for example `001-bootstrap.md`.
- After finishing a phase, update the next-phase prompt rather than rewriting the completed phase as a retrospective.
- When design changes affect future work, update the affected prompt immediately.
- New agents should read `../prompt.md`, `../.memory/*`, and then the newest prompt referenced by `.memory/next.md`.
