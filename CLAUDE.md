# OpenWolf

This project uses OpenWolf for context management. The always-on rules live in `.claude/rules/openwolf.md`; the hooks handle bookkeeping (anatomy index, memory log, read tracking) automatically.

For the full operating protocol (session handoff, memory discipline, bug logging), load the `openwolf` skill, or read `.wolf/OPENWOLF.md`. Regenerate the session handoff with `/handoff`.

# Herdr setup (this harness: Claude Code)

When asked to set up, repair, or verify Herdr pane boxes, pane/tab titles, or the agents list on this machine, follow `herdr/docs/setup-claude-code.md` exactly (claude integration first, plugins, config.toml borders, verification). Do NOT apply steps from the OpenCode guide (`setup-opencode.md`): no `opencode-herdr` plugin, no `small_model` fix. The guide contains a paste-able agent prompt at the end.
