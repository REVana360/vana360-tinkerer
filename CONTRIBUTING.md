# Contributing

Changes to this fork should target `main` and address a focused DAT parser,
resource mapping, audit, export, test, or documentation concern. Preserve the
upstream project's authorship, Rust, frontend, and generated-file conventions.
Do not commit retail DAT files, extracted game data, disc images, credentials,
or private runtime evidence.

## Before submitting

Run `cargo test --all-features`. Resource changes also require the affected
synthetic round-trip or export checks. Retail-backed validation and its reports
remain private.

## Commit subjects

Use `type: imperative summary`: one ASCII line, 50 characters or fewer
including the type, with exactly one space after the colon. Do not use a body,
parentheses, or trailers. Preserve another contributor's credit with Git author
metadata rather than a commit-message trailer.

Choose one type from this fixed list:

- `core` shared DAT parsing and library behavior.
- `cli` command-line interfaces and command orchestration.
- `client` desktop client behavior and presentation.
- `resources` format mappings and distributable project data.
- `build` Cargo, frontend, packaging, and build configuration.
- `deps` dependency and lock-file revisions.
- `tools` repository tooling.
- `docs` documentation and agent guidance.
- `ci` hosted checks and automation.
- `test` test fixtures and harnesses.
- `chore` repository housekeeping with no single code area.
- `refactor` behavior-preserving changes spanning areas.

Prefer the owning area over the kind of change. `landmark`, `feat`, and `fix`
are not types on the maintained branch.
