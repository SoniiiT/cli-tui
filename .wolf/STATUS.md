---
description: session handoff, regenerate with /handoff when a quest finishes
budget_tokens: 1000
---
# STATUS — cli-tui

> Single source of truth for resuming work. Read this FIRST when starting a session.
> Last updated: 2026-10-12

---

## ✅ Done

- **New tool `tools/venv-runner` (Rust/Ratatui), built this session, macOS `supported`; installed at `/opt/homebrew/bin/venv-runner`.** Run in a Python project: finds venvs (any dir with `pyvenv.cfg`, flags broken ones) → **Start** leaves the TUI and runs `<venv>/bin/python <run cmd>` (default `main.py`, editable with `e`, shell-style quoting, `-m uvicorn …` works) with inherited stdio, so `input()`, `pdb`, Ctrl+C work; Enter returns to the TUI. No venv → create form: name (default `.venv`/`.venv-2`), Python (discovered from PATH/pyenv/uv, preselected from `.python-version`, else the real `python3`), dependencies (`requirements*.txt` / `pyproject.toml` / none) → `python -m venv` + `pip install` in the real terminal. **`d` deletes a venv** after a dialog (folder, version, files/size; only `y` confirms, Enter does not); safeguards live in `project::delete_venv` (single path component, must still hold `pyvenv.cfg`, symlinked venv loses only the link).
  - Layout: `src/{project,python,plan}.rs` (lib, pure logic), `src/{app,ui,main}.rs` (TUI). Docs: `README.md`, `tool.toml`, root README catalog row.
  - Verified: 76 unit tests (35 lib + 41 bin, incl. real venv creation), clippy `-D warnings`, fmt, release build; 100 pty checks on the **release** binary (create, start, stdin+pdb, Ctrl+C survives, exit codes, broken venv, delete incl. cancel keys + symlink, path with space+umlaut, q at prompt, terminal restored).
  - Bugs found+fixed by that testing (all in buglog/cerebrum): default interpreter wrong (`Path::file_stem` made `python3.11` match `python3`); idle TUI redrew every 100 ms (now only on change); fixed-height modals clipped the delete dialog's `y/n` line when paths wrap (now `modal_rect` sizes from wrapped content).
- Alias Manager: live alias discovery (`discover_live_aliases`), macOS + Linux Bash/Zsh core path `supported`, binary installed at `/opt/homebrew/bin/alias-manager`. Windows `unsupported`.
- Herdr pane titles/boxes + OpenCode agent-list titles fixed (`herdr/docs/setup-*.md`).

---

## 🚀 Next quest

**Goal:** Try Venv Runner on the user's real project and verify it on Linux.

- The user's example `/home/antonioromeropenin/Development/itc-api-nx-srvc-atsk` is a **Linux path; it does not exist on this Mac**, so the tool has never run against it. Entry point/venv layout of that project are unknown (assumed `main.py`).
- Files: `tools/venv-runner/{tool.toml,README.md}`, root `README.md` (catalog row), `src/project.rs` (`detect_entry`) if the project needs another default.
- Acceptance: on the Linux box `cargo test` + clippy pass, `venv-runner` in that project creates/finds a venv and starts it; then flip Linux to `supported` with evidence in `tool.toml` + both READMEs (see `docs/platform-support.md`). Windows stays `unverified` (Ctrl+C forwarding not implemented there).

### Open decisions (user)
- **Commit?** Nothing is committed; the user said to commit once they are happy after testing. New: `tools/venv-runner/`; modified: `README.md`, `.wolf/{cerebrum,buglog}.json|md` (+ hook-maintained anatomy/memory).
- **Reinstall after changes:** `cargo build --release --manifest-path tools/venv-runner/Cargo.toml && cp tools/venv-runner/target/release/venv-runner /opt/homebrew/bin/` (installed copy is the delete-enabled build; the user is testing it on this Mac).
- Optional idea, not requested: a `uv` backend for venv creation (currently stdlib `venv` on purpose; see cerebrum Decision Log).

---

## Context

- Branch `master`, HEAD `a906d0c`. Uncommitted but **not from this session** (were already dirty): `.claude/settings.json`, `.wolf/config.json`, `.codex/`, `.cursor/`, `GEMINI.md` — don't commit them blindly.
- Not tested: terminal resize, tmux/screen, Linux, Windows.
- macOS quirks: no `tmux`/`timeout`; test TUIs with `pexpect`+`pyte` (method in `.wolf/cerebrum.md` Decision Log); the pyenv shim `python3.12` fails unless that version is active (venv-runner probes by running each candidate).
- Commands: `cargo test|clippy --all-targets -- -D warnings|fmt --check --manifest-path tools/venv-runner/Cargo.toml`

## 📚 References (read IF needed)

- `.wolf/cerebrum.md` (Do-Not-Repeat + Decision Log), `.wolf/buglog.json`, `docs/platform-support.md`
