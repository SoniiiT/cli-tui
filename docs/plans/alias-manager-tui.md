# Implementation plan: Alias Manager TUI

## Goal

Build a standalone fullscreen terminal application for managing custom shell aliases. Users can create, edit, search, and delete aliases with configurable names and commands. Changes are previewed before writing and applied only after explicit confirmation.

## Current state and repository fit

- The repository currently contains conventions and a starter structure for independent tools, but no implemented application or shared toolchain.
- The new tool belongs in its own directory, `tools/alias-manager/`, with a README, `tool.toml`, implementation, and tests.
- Maintain the root catalog and metadata according to `docs/platform-support.md`. Keep platforms `unverified` until their build, installation, and core path have actually been tested.
- Rust with Ratatui and Crossterm is recommended as a concrete, portable TUI implementation option. Confirm the toolchain before implementation; it is not an existing repository requirement.

## Proposed scope

### MVP

- Manage aliases for one or more user-selected shell profiles. Decide the initial shells and operating systems before coding; Bash and Zsh on Linux/macOS are suggested as the starting scope. If Windows is in the MVP, plan PowerShell as a separate adapter and target path.
- Create, edit, and delete names and commands; search live-filters the visible alias list.
- Provide a fullscreen view with the alias list, name, command, search field, and concise context-sensitive key hints.
- Edit aliases in a modal popup above the list. `Enter` on a selected item opens the editor; `Enter` in the editor validates and stages the draft; another `Enter` opens a before/after preview. `y` saves the change and `n` discards it. `Esc` closes the popup without changes.
- Use the same explicit preview and confirmation flow for creation and deletion. Creation previews the new item; deletion previews the item to be removed.
- Keep validation and rendering shell-specific: alias names must be valid for the selected target and commands must not be empty. The application must never execute a configured command when saving it.
- After saving, clearly state which shell configuration was affected and whether a new terminal or a shell-profile reload is required.

### Out of scope for the MVP

- Managing global/root aliases or changing arbitrary system files.
- Managing shell functions, environment variables, PATH entries, or entire shell profiles.
- Executing, testing, or evaluating saved alias commands.
- Synchronization between computers.

## Proposed architecture

1. **TUI and state model:** Separate list/search, edit-popup, and change-preview views. Model navigation and actions with explicit states so `Enter`, `y`, `n`, and `Esc` have unambiguous behavior in every state.
2. **Alias data:** Store normalized user-level records containing at least the target shell, name, and command string in a configuration file under the platform-appropriate user config directory. The format should be versioned and suitable for backups.
3. **Shell adapters:** Validate names per shell and render saved records to a tool-managed file using correct shell escaping. Do not generate shell syntax through unescaped string concatenation.
4. **Activation:** During initial setup, select the target shell and its profile. Preview the integration and add only a clearly marked, idempotent tool-managed section that loads the generated alias file. Back up the profile before each modification and update it atomically; leave all content outside the marked section untouched.
5. **Persistence and error handling:** Write to a temporary file in the same directory, flush it, and rename it atomically. Keep the draft and show a clear message on validation, permission, or write errors; do not partially overwrite existing data.

Storing normalized data and integrating with the shell profile are recommendations for a safe, auditable starting point. Determine exact paths and behavior for each selected shell and platform according to its conventions.

## Implementation steps

1. **Confirm scope and target systems**
   - Decide MVP shells, operating systems, and installation method.
   - Define alias names, quoting, commands, profile paths, and reload behavior for each shell.
   - Include existing profile configuration and duplicate-alias behavior in the requirements.

2. **Create the tool scaffold**
   - Copy `tools/_template/` to `tools/alias-manager/` and adapt the README, `tool.toml`, `src/`, and `tests/` as appropriate.
   - Document the selected language, build/runtime prerequisites, platform statuses, and installation method.
   - Add the tool to the root README catalog and keep its status values synchronized with `tool.toml`.

3. **Implement the data model and shell adapters**
   - Implement the data model, config paths, load/save behavior, schema versioning, and atomic writes.
   - Encapsulate validation, escaping/rendering, and profile integration for each MVP shell.
   - Implement setup and profile changes with preview, backup, and clear error handling.

4. **Implement the TUI and CRUD flows**
   - Implement the fullscreen list, live search, empty/error states, and key hints.
   - Implement create, edit, and delete popups with draft state, preview, and `y`/`n` decisions.
   - Restore terminal state reliably on exit; handle long names/commands and small terminal sizes robustly.

5. **Test and document**
   - Add unit tests for validation, shell rendering, search, and state transitions.
   - Run integration tests with temporary profiles and config directories; preserve profile bytes outside the managed section exactly.
   - Document installation, first-time shell setup, CRUD, activation, backup/recovery, and known platform limitations.
   - Run native build and core-path tests for every target platform, then record `supported` evidence in the metadata and catalog.

## Acceptance criteria

- The tool starts as a usable fullscreen TUI and shows aliases for the selected shell.
- Search filters aliases by name and/or command and lets users select a match for editing.
- Names and commands can be created and edited in a popup; every persistent change has a clear before/after preview.
- The required sequence works: `Enter` opens the editor, `Enter` validates the draft, the next `Enter` opens the preview, `y` applies it, and `n` discards it. `Esc` discards an unsaved draft.
- Deletion requires preview and explicit confirmation; canceling leaves the saved alias unchanged.
- Invalid names and empty commands are rejected before writing; the TUI never executes saved commands.
- Profile integration is repeatable, backs up the profile before changes, and preserves content outside the tool-managed section.
- After saving, an alias is available in the next shell or after the documented target-shell reload.
- The README, `tool.toml`, and root catalog agree; platforms without evidence are not marked supported.

## Risks and mitigations

- **Shell syntax and escaping differ:** isolate shell adapters, test each target explicitly, and never execute commands as part of management.
- **Profiles may be customized:** modify only a marked section, create a backup first, write atomically, and preview the integration.
- **Terminal input differs by platform:** use Crossterm for terminal events and test the core path natively on each claimed operating system.
- **Supporting too many shells expands the MVP:** limit target shells before coding and add further shells as independent adapters.
- **Changes do not automatically affect an already-running parent shell:** communicate behavior clearly and require a new shell or documented reload.

## Open decisions

- Which operating systems and shells must the first release support? Is Windows/PowerShell required?
- Should names and commands be managed for multiple shells at once, or should each run target exactly one profile?
- Should the tool manage only existing aliases, or also install shell integration during first-time setup?
- Is Rust with Ratatui/Crossterm acceptable as an independent toolchain?
- How should existing same-name aliases be handled: overwrite, show a conflict, or allow multiple target profiles?

## Recommended sequence

Decide MVP shells and integration model → create the tool scaffold and metadata → secure the data model and shell adapters → implement the CRUD TUI and previews → complete native platform tests and documentation → record only verified platform statuses in the catalog and metadata.
