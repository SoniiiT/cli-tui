---
description: learned preferences, project conventions, and Do-Not-Repeat rules
budget_tokens: 2000
---
# Cerebrum

> OpenWolf's learning memory. Updated automatically as the AI learns from interactions.
> Do not edit manually unless correcting an error.
> Last updated: 2026-10-08

## User Preferences

<!-- How the user likes things done. Code style, tools, patterns, communication. -->

- Project documentation should be written in English.

## Key Learnings

- **Project:** cli-tui
- **Description:** A collection of standalone CLI and TUI applications for Windows, Linux, and macOS. Each tool addresses its own use case and may use its own language, toolchain, and release strategy. Platform support is verified per tool.
- **Alias Manager:** A child TUI process cannot update aliases in its parent shell. Use a profile-loaded shell function to source generated aliases on exit; an already-running shell needs a one-time profile reload to load that function.
- **Alias Manager — macOS platform status flipped to `supported`:** User independently confirmed live discovery against their real `~/.zshrc`, then asked to mark macOS as supported. Before doing so, ran a full interactive core-path check via `expect`-driven pty input against the release binary (not just the earlier startup/shutdown smoke test): create (type name, Tab, type command, Enter×2 to validate+preview, `y` to confirm) and delete (search filter, `d`, `y`), both verified against the real on-disk `aliases.json`/`zsh-aliases.sh`, which is more reliable than screen-scraping ANSI output. Key `expect` pitfall: an idle-timeout pattern (`-re {.+} { exp_continue }`) hung for 120s because ratatui's redraws kept resetting the idle timer — use fixed `after <ms>` sleeps between scripted keystrokes instead, never idle-timeout loops against a TUI that redraws periodically.
- **Alias Manager — macOS "missing system aliases":** Not a macOS bug. Root cause was that static discovery only text-scans the profile file (and its literal `source`/`.` targets) and never executes it, so it can never see aliases a shell defines internally (e.g. zsh's built-in `run-help=man`, `which-command=whence`, confirmed present even with `zsh -f`) or anything set by conditional/plugin logic. Linux boxes with distro-shipped `.bashrc` aliases happened to work because those are literal text; a hand-rolled macOS `.zshrc` with zero `alias` lines has nothing for the static scanner to find. Fix: added `discover_live_aliases` (runs `shell -i -c alias` in a timeout-bounded child, `tools/alias-manager/src/lib.rs`), merged over the static scan via `merge_discovered_aliases` (live wins on name conflicts) in `main.rs`; this is **additive**, `discover_aliases` itself stays pure/static so its existing hermetic tests are untouched. Verified live on this machine: `zsh -i -c alias` really does return `run-help=man` even via `-f`. Also found and fixed a pre-existing clippy `needless_return` in `default_config_dir`'s macOS/Windows `cfg` branches that no prior session had caught (nobody had run clippy with `--cfg macos` active before).
- **OpenCode titles on Venice:** The title agent (`small_model`) gets `AI_APICallError: Bad Request` from Venice with `z-ai-glm-5-3-flash`, so every session keeps its `New session - …` placeholder; `venice/deepseek-v4-flash` as `small_model` works. Titles generate only for NEW sessions (first exchange), never retroactively. The TUI sets the terminal title to `OC | <title>` once a title exists, which is what Herdr Radar shows in the agents list.
- **Herdr CLI quirks:** `herdr pane process-info` needs `--pane <id>` (not positional); `herdr pane run <id> <cmd>` sends text + Enter; to restart an idle OpenCode TUI in another pane, SIGTERM its process (from `process-info`) and `herdr pane run <id> "opencode --continue"` — sessions persist in opencode.db.

## Do-Not-Repeat

<!-- Mistakes made and corrected. Each entry prevents the same mistake recurring. -->
<!-- Format: [YYYY-MM-DD] Description of what went wrong and what to do instead. -->

## Decision Log

<!-- Significant technical decisions with rationale. Why X was chosen over Y. -->
