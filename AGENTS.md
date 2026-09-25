# Agent Instructions

This project uses Shipshape, a context-isolated spec-driven workflow for coding agents.

Agent opening this project: ensure Shipshape is installed, then load the `shipshape` skill (`shipshape:shipshape` under the plugin channel) and follow its routing before other work. Decide how to involve the human per your configured preferences.

Tooling values such as stack, directories, and commands live in `RIGGING.md`.

## Project specifics

- Rust single-user finger server (RFC 1288), implementation in `src/main.rs`, verification harness in `tests/` (cucumber-rs 0.23).
- The cucumber harness owns scenario selection itself: `CUCUMBER_FILTER_TAGS` env var for tags, `--name` for the scenario (regex first, exact literal fallback, zero-match guard). Do not pass `--tags` and `--name` together.
- `cargo test --test cucumber` runs the suite; the harness spawns the real `thimbl` binary per scenario on an ephemeral port with a temp `HOME`.

Install with the open skills CLI, which works across most agent runtimes:

```bash
npx skills add dmytri/shipshape --skill '*'
```

If your runtime supports the open-plugin format, such as Claude Code or Cursor, install the experimental plugin build instead:

```bash
npx plugins add dmytri/shipshape
```

Update Shipshape at a voyage boundary with `npx skills update` for the skills install, or re-run `npx plugins add dmytri/shipshape` for the plugin build.
