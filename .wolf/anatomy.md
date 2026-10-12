# anatomy.md

> Auto-maintained by OpenWolf. Last scanned: 2026-10-12T20:26:13.662Z
> Files: 31 tracked | Anatomy hits: 0 | Misses: 0

> Project structure index. Auto-maintained by OpenWolf hooks and daemon.
> Run `openwolf scan` to generate, or wait for the first Claude Code session.
> Status: Pending initial scan

## ./

- `AGENTS.md` — OpenWolf (~200 tok)
- `CLAUDE.md` — OpenWolf (~211 tok)
- `README.md` — Project documentation (~762 tok)

## docs/

- `platform-support.md` — Platform support (~704 tok)

## docs/plans/

- `alias-manager-tui.md` — Implementation plan: Alias Manager TUI (~2131 tok)
- `cli-tui-sammlung.md` — Implementation plan: CLI/TUI collection for Windows, Linux, and macOS (~1562 tok)

## herdr/

- `README.md` — Project documentation (~4862 tok)

## herdr/docs/

- `setup-claude-code.md` — Herdr-Setup: Claude Code Harness (~1242 tok)
- `setup-opencode.md` — Herdr-Setup: OpenCode Harness (~1563 tok)

## tools/

- `README.md` — Project documentation (~215 tok)

## tools/_template/

- `README.md` — Project documentation (~171 tok)
- `tool.toml` (~85 tok)

## tools/_template/src/

- `.gitkeep` (~0 tok)

## tools/_template/tests/

- `.gitkeep` (~0 tok)

## tools/alias-manager/

- `.gitignore` — Git ignore rules (~3 tok)
- `Cargo.toml` — Rust package manifest (~91 tok)
- `README.md` — Project documentation (~1565 tok)
- `tool.toml` (~344 tok)

## tools/alias-manager/src/

- `lib.rs` — Alias: as_str, new, matches, changed (~10150 tok)
- `main.rs` — Declares Change (~10145 tok)

## tools/venv-runner/

- `.gitignore` — Git ignore rules (~3 tok)
- `Cargo.toml` — Rust package manifest (~99 tok)
- `README.md` — Project documentation (~1706 tok)
- `tool.toml` (~364 tok)

## tools/venv-runner/src/

- `app.rs` — TUI state and key handling. Nothing here touches the terminal: a key press (~9292 tok)
- `lib.rs` — Core logic for venv-runner: project and venv detection, Python interpreter (~69 tok)
- `main.rs` — Declares HELP (~2419 tok)
- `plan.rs` — Command plans: what to run to create a venv or to start the project in (~3397 tok)
- `project.rs` — Looking at a project directory: which venvs exist, what would run, and (~5794 tok)
- `python.rs` — Finding Python interpreters a new venv can be created from. (~3800 tok)
- `ui.rs` — Rendering. Pure functions of [`App`]; no state changes here. (~7264 tok)
