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

## Do-Not-Repeat

<!-- Mistakes made and corrected. Each entry prevents the same mistake recurring. -->
<!-- Format: [YYYY-MM-DD] Description of what went wrong and what to do instead. -->

## Decision Log

<!-- Significant technical decisions with rationale. Why X was chosen over Y. -->
