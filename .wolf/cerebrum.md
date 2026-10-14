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

- [2026-10-12] Compared executable names with `Path::file_stem()` (`python3.11` → stem `python3`, because `.11` counts as an extension), so every `python3.NN` was taken for `python3`. Compare `file_name()` (minus `.exe`) instead; see `is_command` in `tools/venv-runner/src/python.rs`. Logged in buglog.
- [2026-10-12] A TUI loop that calls `terminal.draw` every poll tick keeps writing escape sequences (ratatui re-emits hide-cursor every frame) even when idle. Redraw only on a change (event, async result, finished child), as `run_app` in `tools/venv-runner/src/main.rs` does.
- [2026-10-12] Ratatui `Paragraph` with `Wrap` silently clips whatever does not fit the fixed-height box, and a long path wraps into several rows. Never size a modal with a fixed row count when it holds paths or messages; size it from the wrapped content (`modal_rect` in `tools/venv-runner/src/ui.rs`) and test at narrow widths with long paths, especially for confirmation dialogs whose last line says which key confirms.
- [2026-10-14] Asserting a multi-word phrase against rendered ratatui output (unit `TestBackend` or pty/pyte) breaks as soon as the text wraps at a space or a long path. It hit four times (key bar, dialogs, preview commands, hint sentences). Assert single words, or flatten first (strip the `│` borders, collapse whitespace) in the pty driver; keep any contiguous-phrase assertions to text that cannot wrap.
- [2026-10-14] Overwrote the installed `/opt/homebrew/bin/venv-runner` with `cp` while 10 of the user's TUI sessions were running it. On macOS arm64 that left a file whose launches got SIGKILL (exit 137, `cmp` against the build differed) and modified the pages under the running processes. Before replacing an installed binary run `pgrep -fl <tool>` and tell the user what is running; replace it with `rm -f` + `install -m 755` (new inode), never `cp` over it. The same `cp` pattern is in the alias-manager install notes.

## Decision Log

<!-- Significant technical decisions with rationale. Why X was chosen over Y. -->

- [2026-10-12] **Venv Runner is Rust/Ratatui, not Python.** It is installed as one binary and must work in any project regardless of the active Python/venv, and every other tool here is Rust. It creates venvs with stdlib `python -m venv` (the same thing the user would type; `uv venv` would skip pip unless `--seed`) and installs with `<venv>/bin/python -m pip`. uv/pyenv Pythons are only *discovered* as interpreters. Possible later addition: a uv backend.
- [2026-10-12] **Start/create run in the real terminal, not inside the TUI.** The TUI leaves the alternate screen, runs the child with inherited stdio, then waits for a key before taking the screen back. That is what makes `input()`, `pdb`, scrollback and Ctrl+C behave normally. While the child runs, a no-op SIGINT *handler* (not `SIG_IGN`, which would be inherited and break `KeyboardInterrupt`) keeps the TUI process alive. Key events are drained before the hand-over so stray keystrokes do not reach the child.
- [2026-10-12] **Testing a TUI on this Mac:** no `tmux`, no `timeout`. Drive the release binary with `pexpect` + `pyte` from a scratch venv, with two `pyte` screens switched on `\x1b[?1049h/l` so the alternate screen and the normal screen can be inspected separately, and assert on rendered text plus on-disk results. Bound every pump loop by a hard cap (an idle ratatui TUI may emit output forever). Only an interactive pty scenario caught the `file_stem` default-interpreter bug; the unit tests had never asserted which interpreter is the default.
- [2026-10-12] **Delete in Venv Runner:** `d` opens a dialog (folder, Python version, file count + size); only `y` deletes (Enter does not). Safety lives in the lib (`delete_venv`), not the UI: single plain path component, must still contain `pyvenv.cfg`, `remove_dir_all` removes only the link for a symlinked venv. Mutation-checked: without the component guard `..` deletes the parent directory.
- [2026-10-14] **Update (`u`) in Venv Runner = the creation recipe applied in place**, not a rebuild: optional `pip install --upgrade pip`, then `pip install [--upgrade --upgrade-strategy eager] <source>` for the detected dependency source (requirements*.txt / `-e .`). Eager is the default because that is what a fresh venv would resolve to. It never uninstalls packages that dropped out of the requirements and never changes the Python version (delete + new for that); a broken venv is refused because pip cannot run in it. The dialog previews the exact commands. Possible later additions: repairing a broken venv with `python -m venv --upgrade` for same-minor interpreters, and an uninstall-extraneous option.
