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

<!-- Move items here from "🚀 Next phase" when finished. Group by area. -->

- Herdr pane structure: `herdr.auto-title` (kryptamine/herdr-auto-title v0.13.2) installed and started; auto-names tabs and panes. Pane boxes enabled natively in `~/.config/herdr/config.toml` (`pane_gaps = true`, `pane_outer_borders = true`, `pane_borders = "always"`); config checked and reloaded. Agents list titles fixed via OpenCode plugin `justmytwospence/opencode-herdr` (pinned f1c0f2b67453) in `opencode.json` + `tui.jsonc`. Root cause for missing titles: Venice rejected title-agent requests for `small_model` z-ai-glm-5-3-flash (Bad Request); switched to `venice/deepseek-v4-flash`, verified end-to-end (new session → generated title → radar row shows `OC | <title>`). Idle OpenCode panes w11/w1A restarted via SIGTERM + `herdr pane run … "opencode --continue"`; the cli-tui pane (wX) still runs the old TUI and needs a user restart. Titles generate only for NEW sessions. Setup split into harness-specific guides: `herdr/docs/setup-claude-code.md` and `herdr/docs/setup-opencode.md`, linked from `herdr/README.md`; both end with a paste-able agent prompt, and `CLAUDE.md`/`AGENTS.md` route each harness to its guide. Learnings in cerebrum, bug in buglog.
- Alias Manager implemented and installed at `~/.cargo/bin/alias-manager`; Bash/Zsh TUI supports discovery of static profile aliases, CRUD, search, preview/confirm, safe storage, and automatic reload through the profile wrapper.
- OpenWolf v2.5.2 initialized for OpenCode in this repository; daemon reports running.
- Repository documentation is in English. Linux Bash core path is supported based on Omarchy 4.0.4/Linux 7.2.5-3-omarchy x86_64 testing; macOS is unverified; Windows is unsupported.

---

## 🚀 Next phase

**Goal:** Await the next user request; no implementation task is currently active.

### Acceptance criteria
1. Confirm scope for the next requested change.

### Files to create / edit
| Type | File | Content |
|---|---|---|
| — | — | No files selected yet. |

### Closed decisions
- Alias activation uses a profile-managed shell function because a child process cannot mutate its parent shell; an already-open shell needs a one-time profile reload.

### Open decisions
- macOS native verification remains outstanding if macOS support is requested.

---

## 📁 Active architecture

- **Stack:** Rust, Ratatui, Crossterm; standalone tool projects.
- **Key modules:** `tools/alias-manager/src/lib.rs` (storage, shell adapters, profile integration), `src/main.rs` (TUI and state flow).
- **Patterns:** Platform support is recorded per tool; statuses must match the root catalog and have native evidence.

---

## ⚠️ External blockers (don't block coding)

- macOS native test is not available in this session; status remains `unverified`.

---

## 🔧 Useful commands

```bash
cargo test --manifest-path tools/alias-manager/Cargo.toml
cargo clippy --manifest-path tools/alias-manager/Cargo.toml --all-targets -- -D warnings
openwolf status
```

---

## 📚 References (read IF needed)

- `.wolf/cerebrum.md` — User Preferences + Do-Not-Repeat + Decision Log
- `.wolf/anatomy.md` — token-efficient file index
- `.wolf/buglog.json` — known bugs + fixes
