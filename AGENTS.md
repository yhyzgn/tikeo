<claude-mem-context>
# Memory Context

# claude-mem status

This project has no memory yet. The current session will seed it; subsequent sessions will receive auto-injected context for relevant past work.

Memory injection starts on your second session in a project.

`/learn-codebase` is available if the user wants to front-load the entire repo into memory in a single pass (~5 minutes on a typical repo, optional). Otherwise memory builds passively as work happens.

Live activity: http://localhost:37777
How it works: `/how-it-works`

This message disappears once the first observation lands.
</claude-mem-context>

## Project architecture / module-entry file convention

- Files that act as module cores or public entry points (for example `mod.rs`, `lib.rs`, `registry.rs`, `dispatcher.rs`, route aggregators, SDK facades, and similar orchestration surfaces) must stay thin and library-like: define public types/contracts, wire submodules, expose APIs, and coordinate high-level flow only.
- Do not let a core/entry file accumulate all implementation details. Split logic by nature, module, and feature responsibility into focused child modules before the file becomes a dumping ground. Typical split boundaries include capability parsing/serialization, persistence adapters, routing/matching/scoring, election/fencing, session identity/generation, protocol binding, validation, and test fixtures.
- Prefer meaningful responsibility boundaries over mechanical line-count appeasement. Source-size checks are a backstop; passing them is not a substitute for keeping entry files cohesive.
- When refactoring an overgrown entry file, preserve behavior with targeted tests first, then move one responsibility at a time, keeping names and module paths clear enough for the next developer to navigate.
