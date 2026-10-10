# GitHub Ops for Herdr (prototype)

One Herdr addon for browsing GitHub issues, pull requests, tags, releases, and Actions runs;
reviewing/editing them; submitting approval reviews; merging with an explicit confirmation;
creating tags and releases; triggering, rerunning, and cancelling workflow runs; and handing
issues, PR reviews, release-notes drafting, or failing-run investigation to an agent in a
separate Herdr worktree.

## Requirements

- Linux or macOS, Herdr 0.9+, Node.js 22+, Git, and the authenticated GitHub CLI (`gh auth login`).
- Rust/Cargo for the bundled GHZinga 0.5.1 viewer. The build installs it inside this plugin's
  `.runtime/` directory; it does not install a second Herdr plugin or globally change Git's pager.

## Build and link for local testing

```bash
bash bin/build-ghzinga.sh
herdr plugin link "$PWD"
herdr config check
```

Open the overlay with `herdr plugin action invoke open --plugin soniiit.github-ops`, or Ctrl-click a
GitHub issue/PR/Actions-run URL in a Herdr pane. The board has Issues, Pull Requests, Tags, Releases,
and Actions modules (`Tab` or `1`–`5` to switch), a filterable list on the left, and tabbed details on
the right. Modules are registered in `lib/modules.mjs` so more sources can be added independently.
Press `v` for GHZinga's full issue/PR viewer and its comment, edit, close/reopen, and merge actions
(issues/PRs only — GHZinga has no tag/release/run support).

`n` on Tags, Releases, and Actions opens a centered form popup with every field visible at once —
`Tab`/`↑↓` move between fields, `←→` cycle a select field's value, typing edits the focused text
(or multi-line text, for `textarea` fields) field, and `Enter` (on the last field) or clicking the
primary button submits. Everything happens in a popup — there is deliberately no bottom-line
`readline` prompt anywhere in this part of the plugin, including the typed safety confirmation
(e.g. `CREATE <name>`) still required afterward for anything that mutates GitHub: it's a one-field
confirmation popup (`openConfirmForm` in `bin/board.mjs`), not a line prompt. Run logs are shown the
same way, in a scrollable read-only popup. (The pre-existing issue/PR Approve/Merge/Comment/Edit/
Close actions predate this and still use line prompts; ask before retrofitting those too.)

**Tags**: the form has Name and Target (branch or SHA) fields; the tag is created via the GitHub API
(`gh` has no `tag` subcommand).

**Releases**: the form has Tag, Title, Notes-source (write manually / generate from merged PRs /
delegate a draft to an AI agent), a multi-line Notes-text field (used only when the source is
"write manually"; `Enter` inserts a newline there, `Tab`/`↑↓` leaves the field), and Visibility
(draft / pre-release / publish). Delegating notes drafting reuses the same worktree+popup pipeline
as issue/PR delegation; the agent only drafts and reports text — it never creates, edits, publishes,
or deletes anything, and GitHub Ops has no way to read an agent pane's output back automatically, so
copy the drafted notes back into a fresh "New Release" manually. `p` publishes a draft, `x` deletes a
release — both require typed confirmation.

**Actions**: the trigger form has Workflow, Ref, and Inputs (`key=value, key2=value2`) fields. `l`
shows failed-step logs; `u`/`f` rerun a run (`f` = failed jobs only); `x` cancels a run — all but `l`
require typed confirmation. The Jobs detail tab lists every job and step with its status, mirroring
the GitHub Actions run page. `d` delegates a failing run to an agent in a worktree checked out at the
run's own commit, with the failed-step log
included in the prompt, for AI-assisted troubleshooting.

Click an issue or PR row to select it, a module label or detail tab to switch views, and an action
button along the bottom to invoke an action. Keyboard navigation remains available. The board's
`approve` and `request changes` actions require a typed confirmation before `gh pr review`. The `d`
menu lists idle/done Herdr agents; select one and choose Continue (or press Enter) to send the issue
or read-only PR review. `Create new` opens clickable menus for PR mode, agent kind and optional
instructions, followed by a task-review window with Start and Cancel buttons. Press `D` in the
agent-kind dropdown to persist the selected default in Herdr's GitHub Ops plugin settings. Start creates a linked
Git worktree and opens OpenCode in a focused, zoomed GitHub Ops Agent popup in the current workspace—
without adding another workspace or lower split. The task is typed but unsent so it can be reviewed
in the popup before pressing Enter. Issue tasks are implementation work. For PRs, choose either a
read-only review or local follow-up changes based on a freshly fetched PR-head ref; follow-up prompts
remain unsent and changes are never pushed by the addon.

No approval, merge, issue write, agent launch, keybinding, pager, or worktree action happens from
background polling. Key features are invoked explicitly in the board.

## Tests

```bash
node --test
```
