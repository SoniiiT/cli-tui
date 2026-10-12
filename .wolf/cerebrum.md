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
- **OpenCode titles on Venice:** The title agent (`small_model`) gets `AI_APICallError: Bad Request` from Venice with `z-ai-glm-5-3-flash`, so every session keeps its `New session - …` placeholder; `venice/deepseek-v4-flash` as `small_model` works. Titles generate only for NEW sessions (first exchange), never retroactively. The TUI sets the terminal title to `OC | <title>` once a title exists, which is what Herdr Radar shows in the agents list.
- **Herdr CLI quirks:** `herdr pane process-info` needs `--pane <id>` (not positional); `herdr pane run <id> <cmd>` sends text + Enter; to restart an idle OpenCode TUI in another pane, SIGTERM its process (from `process-info`) and `herdr pane run <id> "opencode --continue"` — sessions persist in opencode.db.

## Do-Not-Repeat

<!-- Mistakes made and corrected. Each entry prevents the same mistake recurring. -->
<!-- Format: [YYYY-MM-DD] Description of what went wrong and what to do instead. -->

## Decision Log

<!-- Significant technical decisions with rationale. Why X was chosen over Y. -->
