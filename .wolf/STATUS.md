---
description: session handoff, regenerate with /handoff when a quest finishes
budget_tokens: 1000
---
# STATUS — cli-tui

> Single source of truth for resuming work. Read this FIRST when starting a session.
> Update this file at the end of every work phase so the next `/clear` resumes in 1 read.
> Last updated: 2026-10-12

---

## ✅ Done

- **Alias Manager — live alias discovery added, macOS "missing system aliases" root-caused and fixed, macOS now `supported`.** Static scan only reads profile text, so it could never see shell-internal aliases (e.g. zsh's built-in `run-help`) or the user's genuinely-empty `~/.zshrc`. Added `discover_live_aliases` (runs `shell -i -c alias`, 5s timeout) + `merge_discovered_aliases` (live wins) in `tools/alias-manager/src/lib.rs` / `main.rs`; static `discover_aliases` untouched, old tests still pass. Fixed a pre-existing clippy `needless_return` in `default_config_dir`'s macOS branch. User independently confirmed discovery against their real `~/.zshrc`; additionally verified full interactive core path (discover/search/create/delete) via scripted pty (`expect`) input checked against the on-disk `aliases.json`/`zsh-aliases.sh`, not just screen output. macOS flipped `unverified` → `supported` in `tool.toml` + both READMEs (root catalog + tool README). Full detail: `.wolf/buglog.json` → `alias-manager-macos-static-discovery-misses-system-aliases`, `.wolf/cerebrum.md`. Release binary rebuilt and reinstalled to `/opt/homebrew/bin/alias-manager`.
- Alias Manager core (prior session): Bash/Zsh TUI, CRUD, search, preview/confirm, safe storage, profile-managed reload function.
- Herdr pane titles/boxes + OpenCode agent-list titles fixed, documented per-harness in `herdr/docs/setup-*.md`.
- Linux Bash core path `supported` (Omarchy 4.0.4); Windows `unsupported`.

---

## 🚀 Next quest

**Goal:** None active — await the next user request.

### Open decisions
- None currently — macOS platform status resolved this session (see Done).

---

## 📁 Active architecture

- **Stack:** Rust, Ratatui, Crossterm; standalone tool projects.
- **Key modules:** `tools/alias-manager/src/lib.rs` (storage, shell adapters, static + live discovery, profile integration), `tools/alias-manager/src/main.rs` (TUI, state flow, wires `discover_aliases` + `discover_live_aliases` via `merge_discovered_aliases`).
- **Patterns:** Platform support recorded per tool in `tool.toml`, must match root README catalog and carry native evidence (see `docs/platform-support.md`); do not mark a platform `supported` without real native proof of the core interactive path.

---

## ⚠️ External blockers (don't block coding)

- None currently tracked.

---

## 🔧 Useful commands

```bash
cargo test --manifest-path tools/alias-manager/Cargo.toml
cargo clippy --manifest-path tools/alias-manager/Cargo.toml --all-targets -- -D warnings
cargo build --release --manifest-path tools/alias-manager/Cargo.toml  # then cp target/release/alias-manager /opt/homebrew/bin/
openwolf status
```

---

## 📚 References (read IF needed)

- `.wolf/cerebrum.md` — User Preferences + Do-Not-Repeat + Decision Log
- `.wolf/anatomy.md` — token-efficient file index
- `.wolf/buglog.json` — known bugs + fixes
